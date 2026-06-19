use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde_json::json;

use crate::assets::{AssetStore, AssetType, LocalAssetStore};
use crate::classifier::FileClassification;
use crate::config::PipelineConfig;
use crate::extractors::{
    base_document_model, default_confidence, empty_style, stage, update_stats,
};
use crate::model::{
    ContentMode, Diagnostic, DocumentFormat, Element, ElementType, Page, PageProfile, PageType,
    ProcessingStatus,
};
use crate::office::charts::extract_chart_rids;
use crate::office::media::content_type_for_media_path;
use crate::office::notes::{extract_notes_path, extract_notes_text};
use crate::office::ooxml::OoxmlPackage;
use crate::office::pptx::{extract_slide_embed_rids, extract_slide_tables, parse_slide_shape_blocks};
use crate::office::shapes::resolve_embed_target;
use crate::office::slides::{parse_presentation_slide_refs, parse_slide_relationships};
use crate::router::Extractor;
use crate::runtime::output_root_dir;
use crate::tables::{
    TableCell, TableLinearizationOptions, TableStructure, linearize_cells, table_to_csv,
    table_to_html, table_to_markdown,
};
use crate::utils::office_text::normalize_office_text;

#[derive(Default)]
pub struct PptxExtractor;

#[derive(Debug, Clone)]
struct PptxOptions {
    enabled: bool,
    extract_text_boxes: bool,
    extract_titles: bool,
    extract_lists: bool,
    extract_tables: bool,
    extract_images: bool,
    extract_notes: bool,
    extract_shapes: bool,
    extract_charts_metadata: bool,
    synthetic_coordinates: bool,
}

impl Default for PptxOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            extract_text_boxes: true,
            extract_titles: true,
            extract_lists: true,
            extract_tables: true,
            extract_images: true,
            extract_notes: true,
            extract_shapes: true,
            extract_charts_metadata: true,
            synthetic_coordinates: true,
        }
    }
}

impl Extractor for PptxExtractor {
    fn name(&self) -> &'static str {
        "pptx_extractor"
    }

    fn extract(
        &self,
        input_path: &Path,
        classification: &FileClassification,
    ) -> anyhow::Result<crate::model::DocumentModel> {
        let mut model = base_document_model(
            classification,
            DocumentFormat::Pptx,
            ContentMode::Presentation,
            PageType::Slide,
        );
        model.coordinate_system.unit = "synthetic".to_string();
        model.pages.clear();

        let options = read_pptx_options(load_pipeline_config().as_ref());
        if !options.enabled {
            model.warnings.push(diagnostic(
                "PPTX_EXTRACTION_DISABLED",
                "warning",
                "document",
                "PPTX extraction disabled by config",
            ));
            return Ok(model);
        }

        let package = match OoxmlPackage::open(input_path) {
            Ok(pkg) => pkg,
            Err(err) => {
                model.errors.push(diagnostic(
                    "PPTX_INVALID_PACKAGE",
                    "error",
                    "document",
                    &format!("Не удалось открыть PPTX пакет: {}", err),
                ));
                model.processing.status = ProcessingStatus::Failed;
                return Ok(model);
            }
        };

        model
            .processing
            .stages
            .push(stage("pptx_open_package", "pptx_ooxml_parser", 1));

        let slide_refs = match parse_presentation_slide_refs(&package) {
            Ok(v) if !v.is_empty() => v,
            Ok(_) => {
                model.errors.push(diagnostic(
                    "PPTX_PRESENTATION_XML_MISSING",
                    "error",
                    "document",
                    "В PPTX не найдены ссылки на слайды в presentation.xml.",
                ));
                model.processing.status = ProcessingStatus::Failed;
                update_stats(&mut model);
                return Ok(model);
            }
            Err(err) => {
                model.errors.push(diagnostic(
                    "PPTX_PRESENTATION_XML_MISSING",
                    "error",
                    "document",
                    &format!("Не удалось разобрать presentation.xml: {}", err),
                ));
                model.processing.status = ProcessingStatus::Failed;
                update_stats(&mut model);
                return Ok(model);
            }
        };

        model
            .processing
            .stages
            .push(stage("pptx_parse_presentation", "pptx_ooxml_parser", 1));

        let store: Box<dyn AssetStore + Send + Sync> = Box::new(LocalAssetStore::new(output_root_dir()));
        let mut registered_assets = HashSet::new();
        let mut has_tables = false;
        let mut has_images = false;

        for (slide_index, slide_ref) in slide_refs.iter().enumerate() {
            let slide_xml = match package.read_text(&slide_ref.slide_path)? {
                Some(v) => v,
                None => {
                    model.warnings.push(diagnostic(
                        "PPTX_SLIDE_PARSE_FAILED",
                        "warning",
                        "page",
                        &format!("Не найден XML слайда: {}", slide_ref.slide_path),
                    ));
                    continue;
                }
            };

            let rels = parse_slide_relationships(&package, &slide_ref.slide_path);
            let chart_rids = extract_chart_rids(&rels);
            if !chart_rids.is_empty() && options.extract_charts_metadata {
                model.warnings.push(diagnostic(
                    "PPTX_CHART_EXTRACTION_PARTIAL",
                    "warning",
                    "page",
                    "Извлечение метаданных диаграмм PPTX выполнено частично (MVP).",
                ));
            }

            let mut elements = Vec::new();
            let mut local_order = 1u32;
            let mut global_order = 1u32;
            let mut y = 16.0f32;
            let mut emitted_image_rids = HashSet::new();
            let mut text_lines = Vec::new();
            let mut markdown_lines = Vec::new();

            if options.extract_shapes || options.extract_text_boxes {
                for shape in parse_slide_shape_blocks(&slide_xml) {
                    if shape.is_title && !options.extract_titles {
                        continue;
                    }

                    if options.extract_lists && !shape.list_items.is_empty() {
                        let items = shape
                            .list_items
                            .iter()
                            .map(|item| normalize_office_text(item))
                            .filter(|v| !v.is_empty())
                            .collect::<Vec<_>>();
                        if !items.is_empty() {
                            let text = items.join("\n");
                            let markdown = items
                                .iter()
                                .map(|item| format!("- {}", item))
                                .collect::<Vec<_>>()
                                .join("\n");

                            let mut extra = HashMap::new();
                            extra.insert("list_type".to_string(), json!("ul"));
                            extra.insert("items".to_string(), json!(items));

                            elements.push(Element {
                                element_id: format!("p{}_e{}", slide_index + 1, local_order),
                                element_type: ElementType::List,
                                tag: Some("a:p".to_string()),
                                role: Some("slide_list".to_string()),
                                reading_order: Some(local_order),
                                global_order: Some(global_order),
                                bbox: shape_bbox_or_synthetic(&shape, y),
                                polygon: None,
                                content: json!({
                                    "text": text,
                                    "markdown": markdown,
                                    "html": null,
                                    "normalized_text": normalize_office_text(&shape.text),
                                    "raw": shape.text,
                                }),
                                style: empty_style(),
                                provenance: json!({
                                    "method": "native",
                                    "tool": "pptx_ooxml_parser",
                                    "stage": "pptx_extract_text",
                                    "source_ref": {
                                        "kind": "slide",
                                        "value": slide_ref.slide_path,
                                    }
                                }),
                                confidence: default_confidence(),
                                warnings: vec![],
                                extra,
                            });
                            text_lines.push(text);
                            markdown_lines.push(markdown);
                            local_order += 1;
                            global_order += 1;
                            y += 40.0;
                        }
                    }

                    if !shape.text.trim().is_empty() {
                        let normalized = normalize_office_text(&shape.text);
                        let is_title = shape.is_title;
                        let element_type = if is_title {
                            ElementType::Heading
                        } else {
                            ElementType::Text
                        };
                        let role = if is_title { "slide_title" } else { "slide_text" };
                        let markdown = if is_title {
                            format!("# {}", normalized)
                        } else {
                            normalized.clone()
                        };

                        elements.push(Element {
                            element_id: format!("p{}_e{}", slide_index + 1, local_order),
                            element_type,
                            tag: Some("a:t".to_string()),
                            role: Some(role.to_string()),
                            reading_order: Some(local_order),
                            global_order: Some(global_order),
                            bbox: shape_bbox_or_synthetic(&shape, y),
                            polygon: None,
                            content: json!({
                                "text": normalized,
                                "markdown": markdown,
                                "html": null,
                                "normalized_text": normalized,
                                "raw": shape.text,
                            }),
                            style: empty_style(),
                            provenance: json!({
                                "method": "native",
                                "tool": "pptx_ooxml_parser",
                                "stage": "pptx_extract_text",
                                "source_ref": {
                                    "kind": "slide",
                                    "value": slide_ref.slide_path,
                                }
                            }),
                            confidence: default_confidence(),
                            warnings: vec![],
                            extra: HashMap::new(),
                        });

                        text_lines.push(normalized.clone());
                        markdown_lines.push(markdown);
                        local_order += 1;
                        global_order += 1;
                        y += 32.0;
                    }

                    if options.extract_images {
                        for rid in &shape.embed_rids {
                            if !emitted_image_rids.insert(rid.clone()) {
                                continue;
                            }
                            let Some(target) = resolve_embed_target(&rels, rid) else {
                                model.warnings.push(diagnostic(
                                    "PPTX_RELATIONSHIP_MISSING",
                                    "warning",
                                    "page",
                                    &format!("Не найдена связь изображения для {}", rid),
                                ));
                                continue;
                            };
                            let Some(bytes) = package.read_bytes(&target)? else {
                                model.warnings.push(diagnostic(
                                    "PPTX_IMAGE_EXTRACTION_FAILED",
                                    "warning",
                                    "page",
                                    &format!("Не найден media-файл PPTX: {}", target),
                                ));
                                continue;
                            };

                            let file_name = target
                                .split('/')
                                .next_back()
                                .unwrap_or("pptx_image.bin");
                            let mut asset = store.write_asset(
                                &model.document_id,
                                AssetType::EmbeddedImage,
                                &format!("pptx_slide_{}_{}", slide_index + 1, file_name),
                                &bytes,
                                content_type_for_media_path(&target),
                            )?;
                            asset.provenance = json!({
                                "source": "pptx_embedded_image",
                                "tool": "pptx_ooxml_parser",
                                "stage": "pptx_extract_media",
                                "source_ref": {
                                    "kind": "relationship",
                                    "value": rid,
                                }
                            });

                            if registered_assets.insert(asset.asset_id.clone()) {
                                model.assets.push(asset.clone());
                            }
                            has_images = true;

                            let mut extra = HashMap::new();
                            extra.insert("asset_id".to_string(), json!(asset.asset_id));

                            elements.push(Element {
                                element_id: format!("p{}_e{}", slide_index + 1, local_order),
                                element_type: ElementType::Image,
                                tag: Some("a:blip".to_string()),
                                role: Some("embedded_image".to_string()),
                                reading_order: Some(local_order),
                                global_order: Some(global_order),
                                bbox: shape_bbox_or_synthetic(&shape, y),
                                polygon: None,
                                content: json!({
                                    "text": "",
                                    "markdown": "![PPTX image]",
                                    "html": null,
                                    "normalized_text": "",
                                    "raw": target,
                                }),
                                style: empty_style(),
                                provenance: json!({
                                    "method": "native",
                                    "tool": "pptx_ooxml_parser",
                                    "stage": "pptx_extract_media",
                                    "source_ref": {
                                        "kind": "zip_entry",
                                        "value": target,
                                    }
                                }),
                                confidence: default_confidence(),
                                warnings: vec![],
                                extra,
                            });
                            local_order += 1;
                            global_order += 1;
                            y += 56.0;
                        }
                    }
                }

                if options.extract_images {
                    for rid in extract_slide_embed_rids(&slide_xml) {
                        if !emitted_image_rids.insert(rid.clone()) {
                            continue;
                        }
                        let Some(target) = resolve_embed_target(&rels, &rid) else {
                            model.warnings.push(diagnostic(
                                "PPTX_RELATIONSHIP_MISSING",
                                "warning",
                                "page",
                                &format!("Не найдена связь изображения для {}", rid),
                            ));
                            continue;
                        };
                        let Some(bytes) = package.read_bytes(&target)? else {
                            model.warnings.push(diagnostic(
                                "PPTX_IMAGE_EXTRACTION_FAILED",
                                "warning",
                                "page",
                                &format!("Не найден media-файл PPTX: {}", target),
                            ));
                            continue;
                        };

                        let file_name = target
                            .split('/')
                            .next_back()
                            .unwrap_or("pptx_image.bin");
                        let mut asset = store.write_asset(
                            &model.document_id,
                            AssetType::EmbeddedImage,
                            &format!("pptx_slide_{}_{}", slide_index + 1, file_name),
                            &bytes,
                            content_type_for_media_path(&target),
                        )?;
                        asset.provenance = json!({
                            "source": "pptx_embedded_image",
                            "tool": "pptx_ooxml_parser",
                            "stage": "pptx_extract_media",
                            "source_ref": {
                                "kind": "relationship",
                                "value": rid,
                            }
                        });

                        if registered_assets.insert(asset.asset_id.clone()) {
                            model.assets.push(asset.clone());
                        }
                        has_images = true;

                        let mut extra = HashMap::new();
                        extra.insert("asset_id".to_string(), json!(asset.asset_id));

                        elements.push(Element {
                            element_id: format!("p{}_e{}", slide_index + 1, local_order),
                            element_type: ElementType::Image,
                            tag: Some("a:blip".to_string()),
                            role: Some("embedded_image".to_string()),
                            reading_order: Some(local_order),
                            global_order: Some(global_order),
                            bbox: Some([0.0, y, 300.0, y + 180.0]),
                            polygon: None,
                            content: json!({
                                "text": "",
                                "markdown": "![PPTX image]",
                                "html": null,
                                "normalized_text": "",
                                "raw": target,
                            }),
                            style: empty_style(),
                            provenance: json!({
                                "method": "native",
                                "tool": "pptx_ooxml_parser",
                                "stage": "pptx_extract_media",
                                "source_ref": {
                                    "kind": "zip_entry",
                                    "value": target,
                                }
                            }),
                            confidence: default_confidence(),
                            warnings: vec![],
                            extra,
                        });
                        local_order += 1;
                        global_order += 1;
                        y += 56.0;
                    }
                }
            }

            if options.extract_tables {
                for (table_idx, table) in extract_slide_tables(&slide_xml).into_iter().enumerate() {
                    if table.is_empty() {
                        continue;
                    }

                    let rows = table.len();
                    let cols = table.iter().map(|row| row.len()).max().unwrap_or(0);
                    if cols == 0 {
                        continue;
                    }

                    let mut cells = Vec::new();
                    for (r, row) in table.iter().enumerate() {
                        for c in 0..cols {
                            let cell_text = row.get(c).cloned().unwrap_or_default();
                            cells.push(TableCell {
                                row: r,
                                column: c,
                                rowspan: 1,
                                colspan: 1,
                                bbox: None,
                                text: normalize_office_text(&cell_text),
                                html: None,
                                markdown: None,
                                formula: None,
                                is_header: r == 0,
                                confidence: None,
                            });
                        }
                    }

                    let markdown = table_to_markdown(&cells, rows, cols);
                    let csv = table_to_csv(&cells, rows, cols);
                    let html = table_to_html(&cells, rows, cols);
                    let text = cells
                        .iter()
                        .map(|c| c.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" | ");
                    let linearized = linearize_cells(
                        &cells,
                        rows,
                        cols,
                        TableLinearizationOptions {
                            max_rows_per_chunk: 20,
                            language: "ru".to_string(),
                        },
                    );

                    let mut table_element = Element {
                        element_id: format!("p{}_e{}", slide_index + 1, local_order),
                        element_type: ElementType::Table,
                        tag: Some("a:tbl".to_string()),
                        role: Some("slide_table".to_string()),
                        reading_order: Some(local_order),
                        global_order: Some(global_order),
                        bbox: Some([0.0, y, 1000.0, y + (rows as f32 * 24.0)]),
                        polygon: None,
                        content: json!({
                            "text": text,
                            "markdown": markdown,
                            "html": html,
                            "csv": csv,
                            "normalized_text": normalize_office_text(&text),
                            "raw": format!("table_{}", table_idx + 1),
                        }),
                        style: empty_style(),
                        provenance: json!({
                            "method": "native",
                            "tool": "pptx_ooxml_parser",
                            "stage": "pptx_extract_tables",
                            "source_ref": {
                                "kind": "slide",
                                "value": slide_ref.slide_path,
                            }
                        }),
                        confidence: default_confidence(),
                        warnings: vec![],
                        extra: HashMap::new(),
                    };
                    table_element.extra.insert("rows".to_string(), json!(rows));
                    table_element.extra.insert("columns".to_string(), json!(cols));
                    table_element.extra.insert(
                        "cells".to_string(),
                        serde_json::to_value(&cells).unwrap_or_else(|_| json!([])),
                    );
                    table_element.extra.insert(
                        "linearized_chunks".to_string(),
                        serde_json::to_value(&linearized).unwrap_or_else(|_| json!([])),
                    );
                    table_element.extra.insert(
                        "table_structure".to_string(),
                        serde_json::to_value(TableStructure {
                            has_header: true,
                            has_merged_cells: false,
                            orientation: "horizontal".to_string(),
                            extraction_method: "pptx_native".to_string(),
                        })
                        .unwrap_or_else(|_| json!({})),
                    );

                    text_lines.push(text);
                    markdown_lines.push(table_element.content["markdown"].as_str().unwrap_or_default().to_string());
                    elements.push(table_element);
                    local_order += 1;
                    global_order += 1;
                    y += rows as f32 * 24.0 + 16.0;
                    has_tables = true;
                }
            }

            if options.extract_notes {
                if let Some(notes_path) = extract_notes_path(&rels) {
                    match package.read_text(&notes_path)? {
                        Some(notes_xml) => {
                            let notes = extract_notes_text(&notes_xml);
                            if !notes.trim().is_empty() {
                                elements.push(Element {
                                    element_id: format!("p{}_e{}", slide_index + 1, local_order),
                                    element_type: ElementType::Text,
                                    tag: Some("p:notes".to_string()),
                                    role: Some("speaker_notes".to_string()),
                                    reading_order: Some(local_order),
                                    global_order: Some(global_order),
                                    bbox: Some([0.0, y, 1000.0, y + 100.0]),
                                    polygon: None,
                                    content: json!({
                                        "text": notes,
                                        "markdown": notes,
                                        "html": null,
                                        "normalized_text": notes,
                                        "raw": notes_path,
                                    }),
                                    style: empty_style(),
                                    provenance: json!({
                                        "method": "native",
                                        "tool": "pptx_ooxml_parser",
                                        "stage": "pptx_extract_notes",
                                        "source_ref": {
                                            "kind": "notes",
                                            "value": notes_path,
                                        }
                                    }),
                                    confidence: default_confidence(),
                                    warnings: vec![],
                                    extra: HashMap::new(),
                                });
                            }
                        }
                        None => {
                            model.warnings.push(diagnostic(
                                "PPTX_NOTES_SKIPPED",
                                "warning",
                                "page",
                                "Связь notesSlide обнаружена, но notes XML не найден.",
                            ));
                        }
                    }
                }
            }

            let page = Page {
                page_number: (slide_index + 1) as u32,
                page_type: PageType::Slide,
                width: None,
                height: None,
                dpi: None,
                rotation_degrees: 0.0,
                page_image_asset_id: None,
                page_profile: PageProfile {
                    content_mode: ContentMode::Slide,
                    has_native_text: !text_lines.is_empty(),
                    has_ocr_required_regions: false,
                    has_tables,
                    has_images,
                    has_formulas: false,
                    has_handwriting: false,
                    language: Some("ru".to_string()),
                    language_info: crate::language::LanguageInfo::default(),
                    confidence: 0.93,
                },
                elements,
                text: text_lines.join("\n"),
                markdown: markdown_lines.join("\n\n"),
                html: String::new(),
                warnings: vec![],
                extra: {
                    let mut extra = HashMap::new();
                    extra.insert("slide_ref_id".to_string(), json!(slide_ref.rel_id));
                    extra
                },
            };

            model.pages.push(page);
        }

        model.document_profile.has_tables = has_tables;
        model.document_profile.has_images = has_images;
        model.document_profile.has_formulas = false;

        model
            .processing
            .stages
            .push(stage("pptx_parse_slides", "pptx_ooxml_parser", 1));
        model
            .processing
            .stages
            .push(stage("pptx_extract_text", "pptx_ooxml_parser", 1));
        model
            .processing
            .stages
            .push(stage("pptx_extract_tables", "pptx_ooxml_parser", 1));
        model
            .processing
            .stages
            .push(stage("pptx_extract_media", "pptx_ooxml_parser", 1));
        model
            .processing
            .stages
            .push(stage("pptx_extract_notes", "pptx_ooxml_parser", 1));
        model
            .processing
            .stages
            .push(stage("pptx_chunking", "semantic_chunker", 0));

        if model.pages.is_empty() {
            model.errors.push(diagnostic(
                "PPTX_SLIDE_PARSE_FAILED",
                "error",
                "document",
                "Не удалось извлечь ни одного слайда из PPTX.",
            ));
            model.processing.status = ProcessingStatus::Failed;
        }

        update_stats(&mut model);
        model.processing.total_duration_ms = Some(model.processing.stages.len() as u64);
        Ok(model)
    }
}

fn shape_bbox_or_synthetic(shape: &crate::office::pptx::ShapeTextBlock, y: f32) -> Option<[f32; 4]> {
    match (shape.x, shape.y, shape.width, shape.height) {
        (Some(x), Some(y0), Some(w), Some(h)) if w > 0.0 && h > 0.0 => Some([x, y0, x + w, y0 + h]),
        _ => Some([0.0, y, 1000.0, y + 28.0]),
    }
}

fn read_pptx_options(config: Option<&PipelineConfig>) -> PptxOptions {
    let mut options = PptxOptions::default();
    options.enabled = bool_opt(config, &["office", "enabled"], true)
        && bool_opt(config, &["presentation", "pptx", "enabled"], true);
    options.extract_text_boxes = bool_opt(config, &["presentation", "pptx", "extract_text_boxes"], true);
    options.extract_titles = bool_opt(config, &["presentation", "pptx", "extract_titles"], true);
    options.extract_lists = bool_opt(config, &["presentation", "pptx", "extract_lists"], true);
    options.extract_tables = bool_opt(config, &["presentation", "pptx", "extract_tables"], true);
    options.extract_images = bool_opt(config, &["presentation", "pptx", "extract_images"], true);
    options.extract_notes = bool_opt(config, &["presentation", "pptx", "extract_notes"], true);
    options.extract_shapes = bool_opt(config, &["presentation", "pptx", "extract_shapes"], true);
    options.extract_charts_metadata =
        bool_opt(config, &["presentation", "pptx", "extract_charts_metadata"], true);
    options.synthetic_coordinates =
        bool_opt(config, &["presentation", "pptx", "synthetic_coordinates"], true);
    options
}

fn bool_opt(config: Option<&PipelineConfig>, path: &[&str], default: bool) -> bool {
    let Some(cfg) = config else {
        return default;
    };

    let mut cursor = serde_json::to_value(&cfg.pipeline).unwrap_or_else(|_| json!({}));
    for part in path {
        let Some(next) = cursor.get(part) else {
            return default;
        };
        cursor = next.clone();
    }
    cursor.as_bool().unwrap_or(default)
}

fn load_pipeline_config() -> Option<PipelineConfig> {
    crate::config::load_pipeline_config(Path::new("configs/pipeline.config.jsonc")).ok()
}

fn diagnostic(code: &str, severity: &str, scope: &str, message: &str) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity: severity.to_string(),
        scope: scope.to_string(),
        page_number: None,
        element_id: None,
        message: message.to_string(),
        recoverable: true,
        extra: HashMap::new(),
    }
}
