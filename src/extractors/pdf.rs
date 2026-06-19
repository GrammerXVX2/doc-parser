use std::path::Path;
use std::sync::Arc;

use serde_json::{Value, json};

use crate::config::PipelineConfig;
use crate::classifier::FileClassification;
use crate::extractors::{
    base_document_model, stage, update_stats,
};
use crate::merge::{
    DedupOptions, MergeOutcome, merge_native_and_ocr_with_outcome,
};
use crate::model::{
    ContentMode, Diagnostic, DocumentFormat, Page, PageProfile, PageType, ProcessingStage,
    ProcessingStatus, StageStatus,
};
use crate::ocr::{OcrBackendFactory, OcrBackendKind, OcrConfig, OcrPageInput};
use crate::pdf::{
    PdfPageContentMode, classify_pdf_by_text, extract_native_pages, native_text_to_elements,
};
use crate::rendering::{PdfRenderer, RenderedPage};
use crate::runtime::{ocr_cli_overrides, output_root_dir};
use crate::router::Extractor;
use crate::tables::{TableCell, TableStructure, table_to_csv, table_to_html, table_to_markdown};

#[derive(Default)]
pub struct PdfExtractor;

impl Extractor for PdfExtractor {
    fn name(&self) -> &'static str {
        "pdf_extractor"
    }

    fn extract(
        &self,
        input_path: &Path,
        classification: &FileClassification,
    ) -> anyhow::Result<crate::model::DocumentModel> {
        let pipeline_config = load_pipeline_config();
        let extracted = pdf_extract::extract_text(input_path);

        let mut model = base_document_model(
            classification,
            DocumentFormat::Pdf,
            ContentMode::Digital,
            crate::model::PageType::DocumentPage,
        );
        model.coordinate_system.unit = "synthetic".to_string();
        model.pages.clear();

        let text = match extracted {
            Ok(content) => content,
            Err(error) => {
                model.errors.push(Diagnostic {
                    code: "PDF_NATIVE_TEXT_EXTRACTION_FAILED".to_string(),
                    severity: "error".to_string(),
                    scope: "document".to_string(),
                    page_number: None,
                    element_id: None,
                    message: format!("failed to extract native PDF text: {error}"),
                    recoverable: true,
                    extra: Default::default(),
                });
                model.processing.status = ProcessingStatus::Partial;
                String::new()
            }
        };

        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");

        let page_count_hint = lopdf::Document::load(input_path)
            .ok()
            .map(|doc| doc.get_pages().len())
            .filter(|count| *count > 0);

        let pdf_classification = classify_pdf_by_text(
            &normalized,
            classification.is_encrypted_or_protected,
            page_count_hint,
            pipeline_config.as_ref(),
        );

        model.document_profile.content_mode = pdf_classification.document_content_mode.clone();
        model.document_profile.has_native_text = pdf_classification.pages.iter().any(|p| p.has_native_text);
        model.document_profile.has_ocr_required_regions =
            pdf_classification.pages.iter().any(|p| p.requires_ocr);

        model.processing.stages.push(stage(
            "pdf_classification",
            "pdf_page_classifier",
            1,
        ));

        let native_pages = extract_native_pages(&normalized, &pdf_classification);
        model
            .processing
            .stages
            .push(stage("pdf_native_extraction", "pdf_native_extractor", 1));

        let renderer = PdfRenderer::default();
        let mut ocr_config = pipeline_config
            .as_ref()
            .map(|cfg| OcrConfig::from_pipeline_ocr_value(&cfg.pipeline.ocr))
            .unwrap_or_default();
        if let Some(cfg) = &pipeline_config {
            ocr_config.apply_performance_overrides(&cfg.pipeline.performance, Some(&cfg.pipeline.ml));
        }
        if let Some(overrides) = ocr_cli_overrides() {
            ocr_config.apply_cli_overrides(
                overrides.backend.as_deref(),
                overrides.det_model.as_deref(),
                overrides.rec_model.as_deref(),
                overrides.charset.as_deref(),
                overrides.provider.as_deref(),
                overrides.triton_url.as_deref(),
                overrides.save_crops,
            );
        }

        let store: Arc<dyn crate::assets::AssetStore + Send + Sync> =
            Arc::new(crate::assets::LocalAssetStore::new(output_root_dir()));
        let (ocr_pipeline, backend_warnings) = OcrBackendFactory::create(&ocr_config, store)?;
        let effective_backend = if backend_warnings
            .iter()
            .any(|w| w.code == "OCR_BACKEND_FALLBACK_TO_MOCK")
        {
            OcrBackendKind::Mock
        } else {
            ocr_config.backend
        };
        let ocr_tool = match effective_backend {
            OcrBackendKind::Onnx => "onnxruntime",
            OcrBackendKind::Triton => "triton",
            OcrBackendKind::Mock => "mock_ocr",
            OcrBackendKind::Disabled => "ocr_disabled",
        };
        for backend_warning in backend_warnings {
            model.warnings.push(Diagnostic {
                code: backend_warning.code,
                severity: "warning".to_string(),
                scope: "stage".to_string(),
                page_number: None,
                element_id: None,
                message: backend_warning.message,
                recoverable: true,
                extra: Default::default(),
            });
        }

        let dedup_options = dedup_options(pipeline_config.as_ref());

        for page_info in &pdf_classification.pages {
            let page_native_text = native_pages
                .iter()
                .find(|p| p.page_number == page_info.page_number)
                .map(|p| p.text.clone())
                .unwrap_or_default();

            let native_elements = native_text_to_elements(page_info.page_number, &page_native_text);

            let render_needed = should_render_page(page_info.content_mode.clone(), pipeline_config.as_ref());

            let mut rendered_page: Option<RenderedPage> = None;
            if render_needed {
                match crate::pdf::renderer::render_pdf_page_if_needed(
                    &renderer,
                    input_path,
                    page_info.page_number,
                    &model.document_id,
                    pipeline_config.as_ref(),
                ) {
                    Ok(rendered) => {
                        model.assets.push(crate::model::Asset {
                            asset_id: rendered.asset_id.clone(),
                            asset_type: "page_render".to_string(),
                            path: rendered.path.clone(),
                            mime_type: rendered.mime_type.clone(),
                            page_number: Some(page_info.page_number as u32),
                            width: Some(rendered.width),
                            height: Some(rendered.height),
                            dpi: Some(rendered.dpi),
                            sha256: None,
                            provenance: json!({
                                "source": "pdf_page",
                                "tool": "pdf_renderer",
                                "stage": "pdf_render"
                            }),
                            extra: Default::default(),
                        });
                        rendered_page = Some(rendered);
                    }
                    Err(error) => {
                        model.processing.status = ProcessingStatus::Partial;
                        model.errors.push(Diagnostic {
                            code: "PDF_PAGE_RENDER_FAILED".to_string(),
                            severity: "error".to_string(),
                            scope: "page".to_string(),
                            page_number: Some(page_info.page_number as u32),
                            element_id: None,
                            message: format!("failed to render page {}: {error}", page_info.page_number),
                            recoverable: true,
                            extra: Default::default(),
                        });
                    }
                }
            }

            if render_needed {
                model.processing.stages.push(stage("pdf_render", "pdf_renderer", 1));
            } else {
                model.processing.stages.push(ProcessingStage {
                    name: "pdf_render".to_string(),
                    status: StageStatus::Skipped,
                    tool: "pdf_renderer".to_string(),
                    duration_ms: Some(0),
                    metadata: json!({"reason": "digital_page_render_disabled"}),
                });
            }

            let ocr_enabled = ocr_config.enabled && !matches!(effective_backend, OcrBackendKind::Disabled);
            let mut ocr_elements = vec![];
            if page_info.requires_ocr && ocr_enabled {
                if let Some(rendered) = &rendered_page {
                    let input = OcrPageInput {
                        document_id: model.document_id.clone(),
                        page_number: page_info.page_number,
                        image_asset_id: rendered.asset_id.clone(),
                        image_path: Path::new(&output_root_dir())
                            .join(&model.document_id)
                            .join(&rendered.path),
                        width: rendered.width,
                        height: rendered.height,
                        dpi: Some(rendered.dpi),
                    };

                    if matches!(effective_backend, OcrBackendKind::Onnx) {
                        model.processing.stages.push(stage("ocr_load_image", "onnxruntime", 1));
                        model.processing.stages.push(stage("ocr_detection", "onnxruntime", 1));
                        model.processing.stages.push(stage("ocr_crop", "onnxruntime", 1));
                    }

                    ocr_elements = ocr_pipeline.run_page_ocr(input)?;
                    model
                        .processing
                        .stages
                        .push(stage("ocr_recognition", ocr_tool, 1));
                    if matches!(effective_backend, OcrBackendKind::Onnx) {
                        model.processing.stages.push(stage("ocr_total", "onnxruntime", 1));
                    }
                } else {
                    model.processing.stages.push(ProcessingStage {
                        name: "ocr_recognition".to_string(),
                        status: StageStatus::Skipped,
                        tool: ocr_tool.to_string(),
                        duration_ms: Some(0),
                        metadata: json!({"reason": "render_missing_for_ocr"}),
                    });
                }
            } else {
                let reason = if !page_info.requires_ocr {
                    "ocr_not_required"
                } else {
                    "ocr_disabled"
                };
                model.processing.stages.push(ProcessingStage {
                    name: "ocr_recognition".to_string(),
                    status: StageStatus::Skipped,
                    tool: ocr_tool.to_string(),
                    duration_ms: Some(0),
                    metadata: json!({"reason": reason}),
                });
            }

            let MergeOutcome {
                merged_elements,
                removed_ocr_ids,
            } = merge_native_and_ocr_with_outcome(native_elements, ocr_elements, dedup_options.clone());

            for removed_id in removed_ocr_ids {
                model.warnings.push(Diagnostic {
                    code: "OCR_DUPLICATE_REMOVED".to_string(),
                    severity: "warning".to_string(),
                    scope: "element".to_string(),
                    page_number: Some(page_info.page_number as u32),
                    element_id: Some(removed_id),
                    message: "OCR element duplicated native text and was removed.".to_string(),
                    recoverable: true,
                    extra: Default::default(),
                });
            }

            model
                .processing
                .stages
                .push(stage("native_ocr_merge", "native_ocr_merger", 1));

            let page_text = merged_elements
                .iter()
                .filter_map(|e| e.content.get("text").and_then(|v| v.as_str()))
                .collect::<Vec<_>>()
                .join("\n");
            let page_markdown = merged_elements
                .iter()
                .filter_map(|e| e.content.get("markdown").and_then(|v| v.as_str()))
                .collect::<Vec<_>>()
                .join("\n\n");

            let mut merged_elements = merged_elements;
            if page_text.contains('|') {
                let lines = page_text.lines().collect::<Vec<_>>();
                if lines.len() >= 2 {
                    let header = lines[0]
                        .split('|')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(ToOwned::to_owned)
                        .collect::<Vec<_>>();
                    if header.len() >= 2 {
                        let mut rows = vec![header.clone()];
                        for line in lines.iter().skip(1) {
                            let row = line
                                .split('|')
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                                .map(ToOwned::to_owned)
                                .collect::<Vec<_>>();
                            if row.len() == header.len() {
                                rows.push(row);
                            }
                        }

                        if rows.len() >= 2 {
                            let mut cells = Vec::new();
                            for (r, row) in rows.iter().enumerate() {
                                for (c, text) in row.iter().enumerate() {
                                    cells.push(TableCell {
                                        row: r,
                                        column: c,
                                        rowspan: 1,
                                        colspan: 1,
                                        bbox: None,
                                        text: text.clone(),
                                        html: None,
                                        markdown: None,
                                        formula: None,
                                        is_header: r == 0,
                                        confidence: None,
                                    });
                                }
                            }

                            let rows_len = rows.len();
                            let cols_len = header.len();
                            let markdown = table_to_markdown(&cells, rows_len, cols_len);
                            let csv = table_to_csv(&cells, rows_len, cols_len);
                            let html = table_to_html(&cells, rows_len, cols_len);
                            let text = rows
                                .iter()
                                .map(|r| r.join(" | "))
                                .collect::<Vec<_>>()
                                .join("\n");

                            let mut table = crate::model::Element {
                                element_id: format!("p{}_table_1", page_info.page_number),
                                element_type: crate::model::ElementType::Table,
                                tag: Some("pdf_table_stub".to_string()),
                                role: Some("data_table".to_string()),
                                reading_order: Some((merged_elements.len() + 1) as u32),
                                global_order: None,
                                bbox: Some([0.0, 0.0, 1000.0, 200.0]),
                                polygon: None,
                                content: json!({
                                    "text": text,
                                    "markdown": markdown,
                                    "html": html,
                                    "csv": csv,
                                    "normalized_text": text.to_lowercase(),
                                    "raw": page_text,
                                }),
                                style: json!({}),
                                provenance: json!({
                                    "method": "native",
                                    "tool": "pdf_native_extractor",
                                    "stage": "pdf_table_stub"
                                }),
                                confidence: json!({"overall": 0.7, "text": 0.7, "layout": 0.7, "language": 0.7}),
                                warnings: vec![],
                                extra: std::collections::HashMap::new(),
                            };
                            table.extra.insert("rows".to_string(), json!(rows_len));
                            table.extra.insert("columns".to_string(), json!(cols_len));
                            table.extra.insert("cells".to_string(), serde_json::to_value(&cells).unwrap_or_else(|_| json!([])));
                            table.extra.insert(
                                "table_structure".to_string(),
                                serde_json::to_value(TableStructure {
                                    has_header: true,
                                    has_merged_cells: false,
                                    orientation: "horizontal".to_string(),
                                    extraction_method: "pdf_native_stub".to_string(),
                                })
                                .unwrap_or_else(|_| json!({})),
                            );
                            merged_elements.push(table);
                        }
                    }
                }
            }

            let page_type = PageType::DocumentPage;
            let content_mode = match page_info.content_mode {
                PdfPageContentMode::Digital => ContentMode::Digital,
                PdfPageContentMode::Scanned => ContentMode::Scanned,
                PdfPageContentMode::Hybrid => ContentMode::Hybrid,
                PdfPageContentMode::Empty => ContentMode::Unknown,
                PdfPageContentMode::Unknown => ContentMode::Unknown,
            };

            model.pages.push(Page {
                page_number: page_info.page_number as u32,
                page_type,
                width: rendered_page.as_ref().map(|r| r.width as f32),
                height: rendered_page.as_ref().map(|r| r.height as f32),
                dpi: rendered_page.as_ref().map(|r| r.dpi),
                rotation_degrees: 0.0,
                page_image_asset_id: rendered_page.as_ref().map(|r| r.asset_id.clone()),
                page_profile: PageProfile {
                    content_mode,
                    has_native_text: page_info.has_native_text,
                    has_ocr_required_regions: page_info.requires_ocr,
                    has_tables: false,
                    has_images: page_info.has_images,
                    has_formulas: false,
                    has_handwriting: false,
                    language: None,
                    language_info: crate::language::LanguageInfo::default(),
                    confidence: page_info.confidence,
                },
                elements: merged_elements,
                text: page_text,
                markdown: page_markdown,
                html: String::new(),
                warnings: vec![],
                extra: {
                    let mut extra = std::collections::HashMap::new();
                    extra.insert(
                        "pdf_page_classification".to_string(),
                        serde_json::to_value(page_info).unwrap_or_else(|_| Value::Null),
                    );
                    extra
                },
            });
        }

        update_stats(&mut model);
        model.processing.total_duration_ms = Some(model.processing.stages.len() as u64);

        Ok(model)
    }
}

fn load_pipeline_config() -> Option<PipelineConfig> {
    let path = std::path::Path::new("configs/pipeline.config.jsonc");
    crate::config::load_pipeline_config(path).ok()
}

fn should_render_page(mode: PdfPageContentMode, config: Option<&PipelineConfig>) -> bool {
    let render_digital = config
        .and_then(|c| c.pipeline.pdf.get("render_digital_pages"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let render_hybrid = config
        .and_then(|c| c.pipeline.pdf.get("render_hybrid_pages"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let render_scanned = config
        .and_then(|c| c.pipeline.pdf.get("render_scanned_pages"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);

    match mode {
        PdfPageContentMode::Digital => render_digital,
        PdfPageContentMode::Hybrid => render_hybrid,
        PdfPageContentMode::Scanned => render_scanned,
        PdfPageContentMode::Empty | PdfPageContentMode::Unknown => false,
    }
}

fn dedup_options(config: Option<&PipelineConfig>) -> DedupOptions {
    DedupOptions {
        bbox_iou_threshold: config
            .and_then(|c| c.pipeline.merge.get("bbox_iou_threshold"))
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.5) as f32,
        text_similarity_threshold: config
            .and_then(|c| c.pipeline.merge.get("text_similarity_threshold"))
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.8) as f32,
        prefer_native_text: config
            .and_then(|c| c.pipeline.merge.get("prefer_native_text"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true),
    }
}
