use std::path::{Path, PathBuf};
use std::time::Instant;
use std::future::Future;

use anyhow::Context;
use serde_json::json;

use crate::assets::LocalAssetStore;
use crate::classifier::{DetectedFormat, FileClassification, classify_file};
use crate::chunking::SemanticChunker;
use crate::config::{FormatRoutingConfig, PipelineConfig};
use crate::converters::traits::ExtractionContext;
use crate::debug::write_debug_json_asset;
use crate::formulas::{
    DisabledFormulaDetector, DisabledFormulaRecognizer, FixtureFormulaDetector,
    FixtureFormulaRecognizer, FormulaDetectionInput, FormulaDetector, FormulaRecognitionInput,
    FormulaRecognizer, MockFormulaDetector, MockFormulaRecognizer, create_formula_placeholder,
};
use crate::layout::{
    LayoutDetectionInput, LayoutRegion, LayoutSource,
    apply_header_footer_marks, assign_layout_aware_reading_order, build_layout_detector,
    detect_repeated_headers_footers, layout_region_to_element_placeholder,
    resolve_layout_options, should_create_placeholder,
};
use crate::merge::ReadingOrderEngine;
use crate::model::{
    Diagnostic, ElementType, ProcessingStage, ProcessingStatus, StageStatus,
};
use crate::models::books::extract_book_mvp;
use crate::models::backends::mocks::{
    MockDoclingBackend, MockPaddleOcrV6Backend,
};
use crate::models::backends::traits::{
    ExtendedOcrBackend, ExtendedOcrInput, ModelBackend, StructuredDocumentParserBackend,
    StructuredParseInput,
};
use crate::models::config::load_model_stack_config_or_default;
use crate::models::domain::DocumentDomain;
use crate::models::layout::{DoclingLayoutHttpBackend, SuryaLayoutHttpBackend, layout_request};
use crate::models::legal::{extract_legal_mvp, legal_required_fields_present};
use crate::models::ocr::{PaddleOcrV6HttpBackend, SuryaOcrHttpBackend};
use crate::models::router::route_models;
use crate::models::slow_path::decide_slow_path;
use crate::models::structured::DoclingStructuredParseHttpBackend;
use crate::models::tables::{DoclingTableFormerHttpBackend, SuryaTableHttpBackend};
use crate::pdf::{
    PdfTextReconstructionOptions, merge_lines_into_blocks, merge_spans_into_lines,
    pdf_blocks_to_elements, text_to_synthetic_spans,
};
use crate::model::DocumentModel;
use crate::runtime::{output_root_dir, pipeline_cli_overrides};
use crate::tables::{
    DisabledScannedTableDetector, DisabledTableStructureRecognizer, FixtureScannedTableDetector,
    FixtureTableStructureRecognizer, MockScannedTableDetector, MockTableStructureRecognizer,
    ScannedTableDetector, TableDetectionInput, TableStructureInput, TableStructureRecognizer,
    create_scanned_table_placeholder,
};
use crate::utils::geometry::{BBox, bbox_iou};
use crate::pipeline_language::{apply_language_and_locale, localized_message, resolve_locale};
use crate::router::FormatRouter;
use crate::validation::validate_document_model;

pub struct PipelineContext {
    pub pipeline_config: PipelineConfig,
    pub routing_config: FormatRoutingConfig,
}

impl PipelineContext {
    pub fn new(pipeline_config: PipelineConfig, routing_config: FormatRoutingConfig) -> Self {
        Self {
            pipeline_config,
            routing_config,
        }
    }
}

pub fn cli_or_config_bool(
    cli_value: Option<bool>,
    config: Option<&serde_json::Value>,
    default: bool,
) -> bool {
    if let Some(v) = cli_value {
        return v;
    }
    config.and_then(|v| v.as_bool()).unwrap_or(default)
}

pub fn effective_layout_backend(config: &PipelineConfig) -> String {
    let mut backend = config
        .pipeline
        .layout
        .get("backend")
        .and_then(|v| v.as_str())
        .unwrap_or("heuristic")
        .to_string();

    if let Some(overrides) = crate::runtime::pipeline_cli_overrides() {
        if let Some(cli_backend) = &overrides.layout_backend {
            backend = cli_backend.clone();
        }
    }

    backend
}

fn run_async<F>(future: F) -> F::Output
where
    F: Future + Send,
    F::Output: Send,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread {
                tokio::task::block_in_place(|| handle.block_on(future))
            } else {
                std::thread::scope(|scope| {
                    scope
                        .spawn(|| {
                            tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .expect("tokio runtime")
                                .block_on(future)
                        })
                        .join()
                        .expect("scoped async worker thread")
                })
            }
        }
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime")
            .block_on(future),
    }
}

pub fn run_pipeline(input_path: &Path, context: &PipelineContext) -> anyhow::Result<(FileClassification, DocumentModel)> {
    let classification = classify_file(input_path)?;

    let router = FormatRouter::new();
    let extractor = router
        .route(&classification)
        .with_context(|| "failed to route document to extractor")?;

    let mut model = extractor
        .extract(input_path, &classification)
        .with_context(|| format!("extractor '{}' failed", extractor.name()))?;

    if matches!(classification.likely_format, DetectedFormat::Pdf | DetectedFormat::Image) {
        apply_stage7_visual_enhancements(input_path, &classification, context, &mut model);
    } else {
        ReadingOrderEngine::assign_natural_order(&mut model);
    }

    apply_model_stack_enrichment(&classification, &mut model);
    ensure_reading_order(&mut model);

    let max_tokens = context
        .pipeline_config
        .pipeline
        .chunking
        .get("max_tokens_per_chunk")
        .and_then(|v| v.as_u64())
        .unwrap_or(1_000) as usize;
    let chunker = SemanticChunker {
        max_token_estimate: max_tokens,
    };
    model.chunks = chunker.chunk_document(&model);
    model.processing.stages.push(crate::model::ProcessingStage {
        name: "chunking".to_string(),
        status: crate::model::StageStatus::Ok,
        tool: "semantic_chunker".to_string(),
        duration_ms: Some(1),
        metadata: serde_json::json!({}),
    });

    let validation = validate_document_model(&model);
    if !validation.is_empty() {
        model.extra.insert(
            "validation_issues".to_string(),
            serde_json::to_value(validation).unwrap_or_else(|_| serde_json::json!([])),
        );
    }

    // Track selected mode from pipeline config for downstream observability.
    model
        .extra
        .insert("pipeline_mode".to_string(), serde_json::json!(context.pipeline_config.pipeline.mode));
    model.extra.insert(
        "routing_rules_count".to_string(),
        serde_json::json!(context.routing_config.routing.len()),
    );

    apply_language_and_locale(&mut model, &context.pipeline_config);
    let locale = resolve_locale(&context.pipeline_config);
    for warning in &mut model.warnings {
        warning.message = localized_message(locale.clone(), &warning.code, &warning.message);
    }
    for error in &mut model.errors {
        error.message = localized_message(locale.clone(), &error.code, &error.message);
    }
    for page in &mut model.pages {
        for warning in &mut page.warnings {
            warning.message = localized_message(locale.clone(), &warning.code, &warning.message);
        }
        for element in &mut page.elements {
            for warning in &mut element.warnings {
                warning.message = localized_message(locale.clone(), &warning.code, &warning.message);
            }
        }
    }

    Ok((classification, model))
}

fn ensure_reading_order(model: &mut DocumentModel) {
    for page in &mut model.pages {
        for (index, element) in page.elements.iter_mut().enumerate() {
            element.reading_order = Some((index as u32) + 1);
        }
    }
}

fn apply_model_stack_enrichment(classification: &FileClassification, model: &mut DocumentModel) {
    let overrides = pipeline_cli_overrides();
    let config_path = overrides
        .and_then(|o| o.model_stack_config.as_ref())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("configs/model_stack.config.jsonc"));

    if !config_path.exists() {
        model.warnings.push(Diagnostic {
            code: "MODEL_STACK_CONFIG_MISSING".to_string(),
            severity: "warning".to_string(),
            scope: "document".to_string(),
            page_number: None,
            element_id: None,
            message: "Конфиг model stack не найден, используется безопасный профиль по умолчанию.".to_string(),
            recoverable: true,
            extra: Default::default(),
        });
    }

    let config = load_model_stack_config_or_default(Some(config_path.as_path()));
    let routing = route_models(
        classification,
        Some(model),
        &config,
        overrides.and_then(|o| o.model_profile.as_deref()),
        overrides.and_then(|o| o.domain.as_deref()),
    );

    let default_languages = config.model_stack.fallback_languages.clone();

    let mut ocr_backend_used = routing.fast_ocr_backend.clone();
    let mut ocr_backend_type = "mock".to_string();
    let mut ocr_backend_url: Option<String> = None;
    let mut ocr_fallback_used = false;
    let mut ocr_fallback_backend: Option<String> = None;

    let mut layout_backend_used = routing.layout_backend.clone();
    let mut layout_backend_type = "heuristic".to_string();
    let mut layout_backend_url: Option<String> = None;
    let mut layout_fallback_used = false;
    let mut layout_regions_detected = 0usize;

    let structured_backend_used = routing.structured_backend.clone();
    let mut structured_backend_type = "native_or_mock".to_string();
    let mut structured_backend_url: Option<String> = None;
    let mut structured_fallback_used = false;
    let mut structured_executed = false;

    let table_backend_used = routing.table_backend.clone();
    let mut table_fallback_used = false;

    // Stage: model backend health check and initial routing metadata.
    model.processing.stages.push(ProcessingStage {
        name: "model_backend_health_check".to_string(),
        status: StageStatus::Ok,
        tool: "model_stack_router".to_string(),
        duration_ms: Some(1),
        metadata: json!({
            "selected_profile": routing.selected_profile,
            "ocr_backend": routing.fast_ocr_backend,
            "layout_backend": routing.layout_backend,
            "structured_backend": routing.structured_backend,
        }),
    });

    // OCR execution (Paddle primary, Surya fallback, then mock fallback).
    if let Some(ocr_name) = routing.fast_ocr_backend.as_deref() {
        match ocr_name {
            "paddleocr_ppocrv6_medium" => {
                if let Some(cfg) = config.model_stack.backends.get("paddleocr_ppocrv6_medium") {
                    let start = Instant::now();
                    let backend = PaddleOcrV6HttpBackend::new(cfg.clone());
                    let health = run_async(backend.health_check());
                    if health.available {
                        let mut total_regions = 0usize;
                        for page_idx in 0..model.pages.len() {
                            let page_number = model.pages[page_idx].page_number as usize;
                            let image_path = resolve_page_image_path(model, page_idx, std::path::Path::new(&model.source.uri));
                            if image_path.is_none() {
                                continue;
                            }
                            let input = ExtendedOcrInput {
                                document_id: model.document_id.clone(),
                                page_number,
                                image_path: image_path.map(|p| p.to_string_lossy().to_string()),
                                languages: cfg.languages.clone().into_iter().chain(default_languages.clone()).collect(),
                            };
                            let mut ex_ctx = ExtractionContext::default();
                            match run_async(backend.run_ocr(input, &mut ex_ctx)) {
                                Ok(output) => {
                                    total_regions += output.elements.len();
                                    model.pages[page_idx].elements.extend(output.elements);
                                }
                                Err(err) => {
                                    ocr_fallback_used = true;
                                    ocr_fallback_backend = Some("surya_ocr".to_string());
                                    model.warnings.push(Diagnostic {
                                        code: "MODEL_BACKEND_HTTP_ERROR".to_string(),
                                        severity: "warning".to_string(),
                                        scope: "document".to_string(),
                                        page_number: Some(page_number as u32),
                                        element_id: None,
                                        message: format!("PaddleOCR HTTP error, fallback will be used: {err}"),
                                        recoverable: true,
                                        extra: Default::default(),
                                    });
                                }
                            }
                        }

                        ocr_backend_type = "http".to_string();
                        ocr_backend_url = Some(backend.backend_url());
                        model.processing.stages.push(ProcessingStage {
                            name: "paddleocr_http_ocr".to_string(),
                            status: if ocr_fallback_used { StageStatus::Warning } else { StageStatus::Ok },
                            tool: "paddleocr_ppocrv6_medium_http".to_string(),
                            duration_ms: Some(start.elapsed().as_millis() as u64),
                            metadata: json!({
                                "backend_url": ocr_backend_url,
                                "regions": total_regions,
                                "fallback_used": ocr_fallback_used,
                            }),
                        });
                    } else {
                        ocr_fallback_used = true;
                        ocr_fallback_backend = Some("surya_ocr".to_string());
                        model.warnings.push(Diagnostic {
                            code: "PADDLEOCR_SERVICE_UNAVAILABLE".to_string(),
                            severity: "warning".to_string(),
                            scope: "document".to_string(),
                            page_number: None,
                            element_id: None,
                            message: "PaddleOCR service недоступен, будет использован fallback backend.".to_string(),
                            recoverable: true,
                            extra: Default::default(),
                        });
                    }
                }
            }
            "surya_ocr" => {
                if let Some(cfg) = config.model_stack.backends.get("surya_ocr") {
                    let start = Instant::now();
                    let backend = SuryaOcrHttpBackend::new(cfg.clone());
                    let health = run_async(backend.health_check());
                    if health.available {
                        let mut total_regions = 0usize;
                        for page_idx in 0..model.pages.len() {
                            let page_number = model.pages[page_idx].page_number as usize;
                            let image_path = resolve_page_image_path(model, page_idx, std::path::Path::new(&model.source.uri));
                            if image_path.is_none() {
                                continue;
                            }
                            let input = ExtendedOcrInput {
                                document_id: model.document_id.clone(),
                                page_number,
                                image_path: image_path.map(|p| p.to_string_lossy().to_string()),
                                languages: cfg.languages.clone().into_iter().chain(default_languages.clone()).collect(),
                            };
                            let mut ex_ctx = ExtractionContext::default();
                            match run_async(backend.run_ocr(input, &mut ex_ctx)) {
                                Ok(output) => {
                                    total_regions += output.elements.len();
                                    model.pages[page_idx].elements.extend(output.elements);
                                }
                                Err(err) => {
                                    ocr_fallback_used = true;
                                    model.warnings.push(Diagnostic {
                                        code: "MODEL_BACKEND_HTTP_ERROR".to_string(),
                                        severity: "warning".to_string(),
                                        scope: "document".to_string(),
                                        page_number: Some(page_number as u32),
                                        element_id: None,
                                        message: format!("Surya OCR HTTP error, fallback to mock will be used: {err}"),
                                        recoverable: true,
                                        extra: Default::default(),
                                    });
                                }
                            }
                        }

                        ocr_backend_used = Some("surya_ocr".to_string());
                        ocr_backend_type = "http".to_string();
                        ocr_backend_url = Some(backend.backend_url());
                        model.processing.stages.push(ProcessingStage {
                            name: "surya_http_ocr".to_string(),
                            status: if ocr_fallback_used { StageStatus::Warning } else { StageStatus::Ok },
                            tool: "surya_ocr_http".to_string(),
                            duration_ms: Some(start.elapsed().as_millis() as u64),
                            metadata: json!({
                                "backend_url": ocr_backend_url,
                                "regions": total_regions,
                                "fallback_used": ocr_fallback_used,
                            }),
                        });
                    } else {
                        ocr_fallback_used = true;
                        model.warnings.push(Diagnostic {
                            code: "SURYA_SERVICE_UNAVAILABLE".to_string(),
                            severity: "warning".to_string(),
                            scope: "document".to_string(),
                            page_number: None,
                            element_id: None,
                            message: "Surya OCR service недоступен, будет использован mock fallback.".to_string(),
                            recoverable: true,
                            extra: Default::default(),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    if ocr_fallback_used {
        let mut ex_ctx = ExtractionContext::default();
        let mock_backend = MockPaddleOcrV6Backend;
        let mut total_regions = 0usize;
        for page_idx in 0..model.pages.len() {
            let page_number = model.pages[page_idx].page_number as usize;
            let input = ExtendedOcrInput {
                document_id: model.document_id.clone(),
                page_number,
                image_path: None,
                languages: default_languages.clone(),
            };
            if let Ok(output) = run_async(mock_backend.run_ocr(input, &mut ex_ctx)) {
                total_regions += output.elements.len();
                model.pages[page_idx].elements.extend(output.elements);
            }
        }
        model.processing.stages.push(ProcessingStage {
            name: "model_backend_fallback".to_string(),
            status: StageStatus::Warning,
            tool: "mock_ocr".to_string(),
            duration_ms: Some(1),
            metadata: json!({
                "for": "ocr",
                "regions": total_regions,
                "fallback_backend": "mock_paddleocr_v6"
            }),
        });
        model.warnings.push(Diagnostic {
            code: "MODEL_BACKEND_FALLBACK_USED".to_string(),
            severity: "warning".to_string(),
            scope: "document".to_string(),
            page_number: None,
            element_id: None,
            message: "OCR backend fallback activated (mock).".to_string(),
            recoverable: true,
            extra: Default::default(),
        });
        if ocr_backend_used.as_deref() == Some("paddleocr_ppocrv6_medium") {
            ocr_fallback_backend.get_or_insert("mock_paddleocr_v6".to_string());
        }
        ocr_backend_type = "mock".to_string();
    }

    // Layout execution (Surya/Docling HTTP with heuristic fallback).
    if let Some(layout_name) = routing.layout_backend.as_deref() {
        match layout_name {
            "surya_layout" => {
                if let Some(cfg) = config.model_stack.backends.get("surya_layout") {
                    let start = Instant::now();
                    let backend = SuryaLayoutHttpBackend::new(cfg.clone());
                    let health = run_async(backend.health_check());
                    if health.available {
                        let mut all_regions = Vec::new();
                        for page_idx in 0..model.pages.len() {
                            let page_number = model.pages[page_idx].page_number as usize;
                            let request = layout_request(
                                &model.document_id,
                                page_number as u32,
                                resolve_page_image_path(model, page_idx, std::path::Path::new(&model.source.uri))
                                    .map(|p| p.to_string_lossy().to_string()),
                                model.pages[page_idx].width,
                                model.pages[page_idx].height,
                            );
                            match run_async(backend.detect_layout(request)) {
                                Ok(resp) => {
                                    let regions = backend.to_layout_regions(page_number, &resp);
                                    layout_regions_detected += regions.len();
                                    all_regions.extend(regions);
                                }
                                Err(err) => {
                                    layout_fallback_used = true;
                                    model.warnings.push(Diagnostic {
                                        code: "MODEL_BACKEND_HTTP_ERROR".to_string(),
                                        severity: "warning".to_string(),
                                        scope: "page".to_string(),
                                        page_number: Some(page_number as u32),
                                        element_id: None,
                                        message: format!("Surya layout HTTP error: {err}"),
                                        recoverable: true,
                                        extra: Default::default(),
                                    });
                                }
                            }
                        }

                        for region in &all_regions {
                            let page_idx = region.page_number.saturating_sub(1);
                            if page_idx < model.pages.len() {
                                if !should_create_placeholder(&region.region_type) {
                                    continue;
                                }
                                if has_region_duplicate(&model.pages[page_idx], region) {
                                    continue;
                                }
                                model.pages[page_idx]
                                    .elements
                                    .push(layout_region_to_element_placeholder(region));
                            }
                        }

                        layout_backend_type = "http".to_string();
                        layout_backend_url = Some(backend.backend_url());
                        model.processing.stages.push(ProcessingStage {
                            name: "surya_http_layout".to_string(),
                            status: if layout_fallback_used { StageStatus::Warning } else { StageStatus::Ok },
                            tool: "surya_layout_http".to_string(),
                            duration_ms: Some(start.elapsed().as_millis() as u64),
                            metadata: json!({
                                "backend_url": layout_backend_url,
                                "regions": layout_regions_detected,
                                "fallback_used": layout_fallback_used,
                            }),
                        });
                    } else {
                        layout_fallback_used = true;
                        model.warnings.push(Diagnostic {
                            code: "SURYA_SERVICE_UNAVAILABLE".to_string(),
                            severity: "warning".to_string(),
                            scope: "document".to_string(),
                            page_number: None,
                            element_id: None,
                            message: "Surya layout service недоступен, будет использован heuristic fallback.".to_string(),
                            recoverable: true,
                            extra: Default::default(),
                        });
                    }
                }
            }
            "docling_layout" => {
                if let Some(cfg) = config.model_stack.backends.get("docling_layout") {
                    let start = Instant::now();
                    let backend = DoclingLayoutHttpBackend::new(cfg.clone());
                    let health = run_async(backend.health_check());
                    if health.available {
                        let mut all_regions = Vec::new();
                        for page_idx in 0..model.pages.len() {
                            let page_number = model.pages[page_idx].page_number as usize;
                            let request = layout_request(
                                &model.document_id,
                                page_number as u32,
                                resolve_page_image_path(model, page_idx, std::path::Path::new(&model.source.uri))
                                    .map(|p| p.to_string_lossy().to_string()),
                                model.pages[page_idx].width,
                                model.pages[page_idx].height,
                            );
                            match run_async(backend.detect_layout(request)) {
                                Ok(resp) => {
                                    let regions = backend.to_layout_regions(page_number, &resp);
                                    layout_regions_detected += regions.len();
                                    all_regions.extend(regions);
                                }
                                Err(err) => {
                                    layout_fallback_used = true;
                                    model.warnings.push(Diagnostic {
                                        code: "MODEL_BACKEND_HTTP_ERROR".to_string(),
                                        severity: "warning".to_string(),
                                        scope: "page".to_string(),
                                        page_number: Some(page_number as u32),
                                        element_id: None,
                                        message: format!("Docling layout HTTP error: {err}"),
                                        recoverable: true,
                                        extra: Default::default(),
                                    });
                                }
                            }
                        }

                        for region in &all_regions {
                            let page_idx = region.page_number.saturating_sub(1);
                            if page_idx < model.pages.len() {
                                if !should_create_placeholder(&region.region_type) {
                                    continue;
                                }
                                if has_region_duplicate(&model.pages[page_idx], region) {
                                    continue;
                                }
                                model.pages[page_idx]
                                    .elements
                                    .push(layout_region_to_element_placeholder(region));
                            }
                        }

                        layout_backend_used = Some("docling_layout".to_string());
                        layout_backend_type = "http".to_string();
                        layout_backend_url = Some(backend.backend_url());
                        model.processing.stages.push(ProcessingStage {
                            name: "docling_http_layout".to_string(),
                            status: if layout_fallback_used { StageStatus::Warning } else { StageStatus::Ok },
                            tool: "docling_layout_http".to_string(),
                            duration_ms: Some(start.elapsed().as_millis() as u64),
                            metadata: json!({
                                "backend_url": layout_backend_url,
                                "regions": layout_regions_detected,
                                "fallback_used": layout_fallback_used,
                            }),
                        });
                    } else {
                        layout_fallback_used = true;
                        model.warnings.push(Diagnostic {
                            code: "DOCLING_SERVICE_UNAVAILABLE".to_string(),
                            severity: "warning".to_string(),
                            scope: "document".to_string(),
                            page_number: None,
                            element_id: None,
                            message: "Docling layout service недоступен, будет использован heuristic fallback.".to_string(),
                            recoverable: true,
                            extra: Default::default(),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    if layout_fallback_used {
        model.processing.stages.push(ProcessingStage {
            name: "model_backend_fallback".to_string(),
            status: StageStatus::Warning,
            tool: "heuristic_layout".to_string(),
            duration_ms: Some(1),
            metadata: json!({
                "for": "layout",
                "fallback_backend": "heuristic_layout"
            }),
        });
        model.warnings.push(Diagnostic {
            code: "MODEL_BACKEND_FALLBACK_USED".to_string(),
            severity: "warning".to_string(),
            scope: "document".to_string(),
            page_number: None,
            element_id: None,
            message: "Layout backend fallback activated (heuristic).".to_string(),
            recoverable: true,
            extra: Default::default(),
        });
        layout_backend_type = "heuristic".to_string();
    }

    // Structured parsing (Docling HTTP with mock fallback).
    if let Some(structured_name) = routing.structured_backend.as_deref() {
        if structured_name == "docling" {
            if let Some(cfg) = config.model_stack.backends.get("docling") {
                let start = Instant::now();
                let backend = DoclingStructuredParseHttpBackend::new(cfg.clone());
                let health = run_async(backend.health_check());
                if health.available {
                    let mut ex_ctx = ExtractionContext::default();
                    let input = StructuredParseInput {
                        document_id: model.document_id.clone(),
                        input_path: model.source.uri.clone(),
                    };
                    match run_async(backend.parse_document_structured(input, &mut ex_ctx)) {
                        Ok(out) => {
                            structured_executed = out.executed;
                            structured_backend_type = "http".to_string();
                            structured_backend_url = Some(backend.backend_url());
                            if let Some(markdown) = out.metadata.get("markdown").and_then(|v| v.as_str()) {
                                if let Some(page) = model.pages.first_mut() {
                                    if page.markdown.trim().is_empty() {
                                        page.markdown = markdown.to_string();
                                    }
                                }
                            }
                            if let Some(text) = out.metadata.get("text").and_then(|v| v.as_str()) {
                                if let Some(page) = model.pages.first_mut() {
                                    if page.text.trim().is_empty() {
                                        page.text = text.to_string();
                                    }
                                }
                            }
                            model.extra.insert(
                                "docling_structured".to_string(),
                                out.metadata.clone(),
                            );
                        }
                        Err(err) => {
                            structured_fallback_used = true;
                            model.warnings.push(Diagnostic {
                                code: "MODEL_BACKEND_HTTP_ERROR".to_string(),
                                severity: "warning".to_string(),
                                scope: "document".to_string(),
                                page_number: None,
                                element_id: None,
                                message: format!("Docling parse HTTP error, fallback to mock will be used: {err}"),
                                recoverable: true,
                                extra: Default::default(),
                            });
                        }
                    }

                    model.processing.stages.push(ProcessingStage {
                        name: "docling_http_parse".to_string(),
                        status: if structured_fallback_used { StageStatus::Warning } else { StageStatus::Ok },
                        tool: "docling_http".to_string(),
                        duration_ms: Some(start.elapsed().as_millis() as u64),
                        metadata: json!({
                            "backend_url": structured_backend_url,
                            "executed": structured_executed,
                            "fallback_used": structured_fallback_used,
                        }),
                    });
                } else {
                    structured_fallback_used = true;
                    model.warnings.push(Diagnostic {
                        code: "DOCLING_SERVICE_UNAVAILABLE".to_string(),
                        severity: "warning".to_string(),
                        scope: "document".to_string(),
                        page_number: None,
                        element_id: None,
                        message: "Docling service недоступен, будет использован mock/native fallback.".to_string(),
                        recoverable: true,
                        extra: Default::default(),
                    });
                }
            }
        }
    }

    if structured_fallback_used {
        let mut ex_ctx = ExtractionContext::default();
        let mock_backend = MockDoclingBackend;
        let input = StructuredParseInput {
            document_id: model.document_id.clone(),
            input_path: model.source.uri.clone(),
        };
        if let Ok(out) = run_async(mock_backend.parse_document_structured(input, &mut ex_ctx)) {
            structured_executed = out.executed;
            model.extra.insert("docling_structured".to_string(), out.metadata);
        }
        model.processing.stages.push(ProcessingStage {
            name: "model_backend_fallback".to_string(),
            status: StageStatus::Warning,
            tool: "mock_docling".to_string(),
            duration_ms: Some(1),
            metadata: json!({
                "for": "structured_document_parse",
                "fallback_backend": "mock_docling"
            }),
        });
        model.warnings.push(Diagnostic {
            code: "MODEL_BACKEND_FALLBACK_USED".to_string(),
            severity: "warning".to_string(),
            scope: "document".to_string(),
            page_number: None,
            element_id: None,
            message: "Structured parser fallback activated (mock_docling).".to_string(),
            recoverable: true,
            extra: Default::default(),
        });
        structured_backend_type = "mock".to_string();
    }

    // Optional table helper health checks for explicit table backends.
    if let Some(table_name) = routing.table_backend.as_deref() {
        match table_name {
            "surya_table" => {
                if let Some(cfg) = config.model_stack.backends.get("surya_table") {
                    let backend = SuryaTableHttpBackend::new(cfg.clone());
                    let health = run_async(backend.health_check());
                    if !health.available {
                        table_fallback_used = true;
                        model.warnings.push(Diagnostic {
                            code: "SURYA_SERVICE_UNAVAILABLE".to_string(),
                            severity: "warning".to_string(),
                            scope: "document".to_string(),
                            page_number: None,
                            element_id: None,
                            message: "Surya table service недоступен, будет использован placeholder fallback.".to_string(),
                            recoverable: true,
                            extra: Default::default(),
                        });
                    }
                }
            }
            "docling_tableformer" => {
                if let Some(cfg) = config.model_stack.backends.get("docling_tableformer") {
                    let backend = DoclingTableFormerHttpBackend::new(cfg.clone());
                    let health = run_async(backend.health_check());
                    if !health.available {
                        table_fallback_used = true;
                        model.warnings.push(Diagnostic {
                            code: "DOCLING_SERVICE_UNAVAILABLE".to_string(),
                            severity: "warning".to_string(),
                            scope: "document".to_string(),
                            page_number: None,
                            element_id: None,
                            message: "Docling table service недоступен, будет использован placeholder fallback.".to_string(),
                            recoverable: true,
                            extra: Default::default(),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    for reason in &routing.reasons {
        if reason.starts_with("MODEL_PROFILE_NOT_FOUND") {
            model.warnings.push(Diagnostic {
                code: "MODEL_PROFILE_NOT_FOUND".to_string(),
                severity: "warning".to_string(),
                scope: "document".to_string(),
                page_number: None,
                element_id: None,
                message: "Запрошенный профиль моделей не найден, использован fallback профиль.".to_string(),
                recoverable: true,
                extra: Default::default(),
            });
        }
        if reason.starts_with("OCR_BACKEND_FALLBACK_USED") {
            model.warnings.push(Diagnostic {
                code: "OCR_BACKEND_FALLBACK_USED".to_string(),
                severity: "warning".to_string(),
                scope: "document".to_string(),
                page_number: None,
                element_id: None,
                message: "Основной OCR backend недоступен, использован fallback.".to_string(),
                recoverable: true,
                extra: Default::default(),
            });
        }
    }

    let force_legal = overrides.and_then(|o| o.legal_extract);
    let force_book = overrides.and_then(|o| o.book_extract);

    let should_legal_extract = force_legal.unwrap_or_else(|| {
        routing.selected_profile.starts_with("legal")
            || matches!(routing.domain_profile.domain, DocumentDomain::Legal)
    });

    if should_legal_extract {
        let legal = extract_legal_mvp(model);
        if config.model_stack.routing.legal_required_fields_check
            && !legal_required_fields_present(&legal)
        {
            model.warnings.push(Diagnostic {
                code: "LEGAL_REQUIRED_FIELDS_MISSING".to_string(),
                severity: "warning".to_string(),
                scope: "document".to_string(),
                page_number: None,
                element_id: None,
                message: "Не найдены все обязательные юридические поля (parties/dates/identifiers)."
                    .to_string(),
                recoverable: true,
                extra: Default::default(),
            });
        }
        model.extra.insert(
            "legal".to_string(),
            serde_json::to_value(&legal).unwrap_or_else(|_| json!({})),
        );
    }

    let should_book_extract = force_book.unwrap_or_else(|| {
        routing.selected_profile.starts_with("fiction")
            || matches!(
                routing.domain_profile.domain,
                DocumentDomain::Fiction | DocumentDomain::HistoricalBook
            )
    });

    if should_book_extract {
        let book = extract_book_mvp(model);
        if book.dehyphenation_applied {
            model.warnings.push(Diagnostic {
                code: "BOOK_DEHYPHENATION_APPLIED".to_string(),
                severity: "warning".to_string(),
                scope: "document".to_string(),
                page_number: None,
                element_id: None,
                message: "Применена де-гипенизация переносов в книжном тексте.".to_string(),
                recoverable: true,
                extra: Default::default(),
            });
        }
        if book.historical_orthography_detected {
            model.warnings.push(Diagnostic {
                code: "HISTORICAL_ORTHOGRAPHY_DETECTED".to_string(),
                severity: "warning".to_string(),
                scope: "document".to_string(),
                page_number: None,
                element_id: None,
                message: "Обнаружена дореформенная орфография в тексте книги.".to_string(),
                recoverable: true,
                extra: Default::default(),
            });
        }
        model.extra.insert(
            "book".to_string(),
            serde_json::to_value(&book).unwrap_or_else(|_| json!({})),
        );
    }

    model.extra.insert(
        "domain_profile".to_string(),
        serde_json::to_value(&routing.domain_profile).unwrap_or_else(|_| json!({})),
    );

    let slow_path = decide_slow_path(model, &routing, &config);
    if slow_path.should_run {
        model.warnings.push(Diagnostic {
            code: "SLOW_PATH_TRIGGERED".to_string(),
            severity: "warning".to_string(),
            scope: "document".to_string(),
            page_number: None,
            element_id: None,
            message: "Условия slow path выполнены, решение записано в model_outputs.".to_string(),
            recoverable: true,
            extra: Default::default(),
        });
    }

    let selected_profile_cfg = config
        .model_stack
        .profiles
        .get(&routing.selected_profile)
        .cloned()
        .unwrap_or_default();
    let primary_ocr = selected_profile_cfg
        .ocr
        .get("primary")
        .and_then(|v| {
            v.get("backend")
                .and_then(|x| x.as_str())
                .map(ToOwned::to_owned)
                .or_else(|| v.as_str().map(ToOwned::to_owned))
        })
        .or_else(|| {
            selected_profile_cfg
                .ocr
                .as_str()
                .map(ToOwned::to_owned)
        });
    let fallback_used = routing.fast_ocr_backend != primary_ocr;

    let ocr_backend_cfg = ocr_backend_used
        .as_ref()
        .and_then(|b| config.model_stack.backends.get(b));
    let ocr_detection_model = ocr_backend_cfg.and_then(|b| b.detection_model.clone());
    let ocr_recognition_model = ocr_backend_cfg.and_then(|b| b.recognition_model.clone());
    let ocr_languages = ocr_backend_cfg
        .map(|b| b.languages.clone())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| config.model_stack.fallback_languages.clone());

    let table_count = model
        .pages
        .iter()
        .flat_map(|p| p.elements.iter())
        .filter(|e| e.element_type == ElementType::Table)
        .count() as u32;
    let formula_count = model
        .pages
        .iter()
        .flat_map(|p| p.elements.iter())
        .filter(|e| e.element_type == ElementType::Formula)
        .count() as u32;
    let placeholder_tables = model
        .pages
        .iter()
        .flat_map(|p| p.elements.iter())
        .filter(|e| {
            e.element_type == ElementType::Table
                && e.content
                    .get("text")
                    .and_then(|v| v.as_str())
                    .map(|s| s.contains("placeholder"))
                    .unwrap_or(false)
        })
        .count() as u32;
    let placeholder_formulas = model
        .pages
        .iter()
        .flat_map(|p| p.elements.iter())
        .filter(|e| {
            e.element_type == ElementType::Formula
                && e.extra
                    .get("format")
                    .and_then(|v| v.as_str())
                    .map(|v| v == "unknown")
                    .unwrap_or(false)
        })
        .count() as u32;

    let legal_count = model
        .extra
        .get("legal")
        .and_then(|v| v.get("identifiers"))
        .and_then(|v| v.as_array())
        .map(|v| v.len() as u32)
        .unwrap_or(0);

    let model_outputs = json!({
        "routing": {
            "selected_profile": routing.selected_profile,
            "domain": routing.domain_profile.domain.as_str(),
            "confidence": routing.domain_profile.confidence,
            "reasons": routing.domain_profile.reasons,
        },
        "ocr": {
            "backend": ocr_backend_used,
            "backend_type": ocr_backend_type,
            "url": ocr_backend_url,
            "detection_model": ocr_detection_model,
            "recognition_model": ocr_recognition_model,
            "fallback_used": fallback_used || ocr_fallback_used,
            "fallback_backend": ocr_fallback_backend,
            "languages": ocr_languages,
            "pages_processed": model.pages.len(),
            "elements_created": model.stats.ocr_element_count,
        },
        "layout": {
            "backend": layout_backend_used,
            "backend_type": layout_backend_type,
            "url": layout_backend_url,
            "fallback_used": layout_fallback_used,
            "regions_detected": layout_regions_detected,
        },
        "structured_document_parse": {
            "backend": structured_backend_used,
            "backend_type": structured_backend_type,
            "url": structured_backend_url,
            "fallback_used": structured_fallback_used,
            "executed": structured_executed,
        },
        "tables": {
            "backend": table_backend_used,
            "fallback_used": table_fallback_used,
            "native_tables": table_count,
            "scanned_tables": table_count,
            "placeholder_tables": placeholder_tables,
        },
        "formulas": {
            "backend": routing.formula_backend,
            "native_formulas": formula_count,
            "scanned_formulas": formula_count,
            "placeholder_formulas": placeholder_formulas,
        },
        "legal": {
            "ner_backend": routing.legal_ner_backend,
            "embedding_backend": routing.embedding_backend,
            "entities_extracted": model.extra.contains_key("legal"),
            "entities_count": legal_count,
        },
        "slow_path": {
            "enabled": config.model_stack.routing.allow_slow_path,
            "triggered": slow_path.should_run,
            "backend": slow_path.backend,
            "alternatives": slow_path.alternatives,
            "executed": slow_path.executed,
            "reason": slow_path.reasons.first().cloned(),
        }
    });

    model.extra.insert("model_outputs".to_string(), model_outputs);
    model.processing.stages.push(ProcessingStage {
        name: "model_routing".to_string(),
        status: StageStatus::Ok,
        tool: "model_stack_router".to_string(),
        duration_ms: Some(1),
        metadata: json!({
            "selected_profile": model
                .extra
                .get("model_outputs")
                .and_then(|v| v.get("routing"))
                .and_then(|v| v.get("selected_profile"))
                .cloned()
                .unwrap_or(json!("mixed_enterprise")),
        }),
    });
}

fn apply_stage7_visual_enhancements(
    input_path: &Path,
    classification: &FileClassification,
    context: &PipelineContext,
    model: &mut DocumentModel,
) {
    apply_pdf_text_reconstruction(classification, &context.pipeline_config, model);

    let mut layout_regions = Vec::<LayoutRegion>::new();
    let layout_options = resolve_layout_options(Some(&context.pipeline_config));

    if layout_options.enabled {
        let detector = build_layout_detector(&layout_options);
        let mut total_layout_regions = 0usize;

        for page_idx in 0..model.pages.len() {
            let page_number = model.pages[page_idx].page_number as usize;
            let page_width = model.pages[page_idx].width.unwrap_or(1000.0);
            let page_height = model.pages[page_idx].height.unwrap_or(1400.0);
            let page_image_path = resolve_page_image_path(model, page_idx, input_path);
            let page_elements = model.pages[page_idx].elements.clone();

            let input = LayoutDetectionInput {
                document_id: model.document_id.clone(),
                page_number,
                page_image_asset_id: model.pages[page_idx].page_image_asset_id.clone(),
                page_image_path,
                page_width,
                page_height,
                existing_elements: page_elements,
            };

            let mut ex_ctx = ExtractionContext::default();
            match run_async(detector.detect_layout(input, &mut ex_ctx)) {
                Ok(mut regions) => {
                    total_layout_regions += regions.len();
                    for region in &regions {
                        if !should_create_placeholder(&region.region_type) {
                            continue;
                        }
                        if has_region_duplicate(&model.pages[page_idx], region) {
                            continue;
                        }
                        model.pages[page_idx]
                            .elements
                            .push(layout_region_to_element_placeholder(region));
                    }
                    layout_regions.append(&mut regions);
                }
                Err(err) => {
                    model.warnings.push(Diagnostic {
                        code: "LAYOUT_DETECTION_FAILED".to_string(),
                        severity: "warning".to_string(),
                        scope: "page".to_string(),
                        page_number: Some(page_number as u32),
                        element_id: None,
                        message: err.message,
                        recoverable: true,
                        extra: Default::default(),
                    });
                    model.processing.status = ProcessingStatus::Partial;
                }
            }
        }

        model.processing.stages.push(ProcessingStage {
            name: "layout_detection".to_string(),
            status: StageStatus::Ok,
            tool: format!("{}_layout_detector", effective_layout_backend(&context.pipeline_config)),
            duration_ms: Some(1),
            metadata: json!({"regions": total_layout_regions}),
        });
    } else {
        model.processing.stages.push(ProcessingStage {
            name: "layout_detection".to_string(),
            status: StageStatus::Skipped,
            tool: "layout_detector".to_string(),
            duration_ms: Some(0),
            metadata: json!({"reason": "layout_disabled"}),
        });
    }

    apply_scanned_table_pipeline(input_path, &context.pipeline_config, model);
    apply_formula_pipeline(input_path, &context.pipeline_config, model);

    if layout_options.detect_headers_footers {
        let detected = detect_repeated_headers_footers(&model.pages);
        apply_header_footer_marks(&mut model.pages, &detected);
        model.processing.stages.push(ProcessingStage {
            name: "header_footer_detection".to_string(),
            status: StageStatus::Ok,
            tool: "header_footer_detector".to_string(),
            duration_ms: Some(1),
            metadata: json!({
                "headers": detected.header_element_ids.len(),
                "footers": detected.footer_element_ids.len(),
            }),
        });
    }

    match assign_layout_aware_reading_order(
        &mut model.pages,
        &layout_regions,
        layout_options.reading_order.clone(),
    ) {
        Ok(()) => {
            model.processing.stages.push(ProcessingStage {
                name: "layout_aware_reading_order".to_string(),
                status: StageStatus::Ok,
                tool: "layout_reading_order".to_string(),
                duration_ms: Some(1),
                metadata: json!({"regions": layout_regions.len()}),
            });
        }
        Err(err) => {
            model.warnings.push(Diagnostic {
                code: "READING_ORDER_FAILED".to_string(),
                severity: "warning".to_string(),
                scope: "document".to_string(),
                page_number: None,
                element_id: None,
                message: format!("Не удалось применить layout-aware reading order: {err}"),
                recoverable: true,
                extra: Default::default(),
            });
            ReadingOrderEngine::assign_natural_order(model);
        }
    }

    write_layout_debug_artifacts_if_needed(
        input_path,
        &context.pipeline_config,
        model,
        &layout_regions,
        layout_options.save_debug_artifacts,
    );
}

fn apply_pdf_text_reconstruction(
    classification: &FileClassification,
    config: &PipelineConfig,
    model: &mut DocumentModel,
) {
    if !matches!(classification.likely_format, DetectedFormat::Pdf) {
        return;
    }

    let enabled = config
        .pipeline
        .pdf
        .get("text_reconstruction")
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if !enabled {
        return;
    }

    let cfg = config.pipeline.pdf.get("text_reconstruction");
    let mut options = PdfTextReconstructionOptions::default();
    if let Some(value) = cfg {
        if let Some(v) = value.get("line_y_tolerance").and_then(|v| v.as_f64()) {
            options.line_y_tolerance = v as f32;
        }
        if let Some(v) = value.get("word_gap_ratio").and_then(|v| v.as_f64()) {
            options.word_gap_ratio = v as f32;
        }
        if let Some(v) = value.get("paragraph_gap_ratio").and_then(|v| v.as_f64()) {
            options.paragraph_gap_ratio = v as f32;
        }
        if let Some(v) = value.get("detect_headings_by_font").and_then(|v| v.as_bool()) {
            options.detect_headings_by_font = v;
        }
    }

    for page in &mut model.pages {
        if page.text.trim().is_empty() {
            continue;
        }
        let spans = text_to_synthetic_spans(page.page_number as usize, &page.text);
        let lines = merge_spans_into_lines(spans, options.clone());
        let blocks = merge_lines_into_blocks(lines, options.clone());
        let reconstructed = pdf_blocks_to_elements(blocks);

        if reconstructed.is_empty() {
            continue;
        }

        let mut retained = page
            .elements
            .iter()
            .filter(|e| {
                !matches!(
                    e.element_type,
                    ElementType::Text | ElementType::Heading | ElementType::Paragraph
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        retained.extend(reconstructed);
        page.elements = retained;
    }

    model.processing.stages.push(ProcessingStage {
        name: "pdf_text_reconstruction".to_string(),
        status: StageStatus::Ok,
        tool: "pdf_block_reconstructor".to_string(),
        duration_ms: Some(1),
        metadata: json!({}),
    });
}

fn apply_scanned_table_pipeline(input_path: &Path, config: &PipelineConfig, model: &mut DocumentModel) {
    let enabled = cli_or_config_bool(
        pipeline_cli_overrides().and_then(|o| o.detect_scanned_tables),
        config.pipeline.scanned_tables.get("enabled"),
        true,
    );
    if !enabled {
        model.processing.stages.push(ProcessingStage {
            name: "scanned_table_detection".to_string(),
            status: StageStatus::Skipped,
            tool: "scanned_table_detector".to_string(),
            duration_ms: Some(0),
            metadata: json!({"reason": "disabled"}),
        });
        return;
    }

    let backend = config
        .pipeline
        .scanned_tables
        .get("backend")
        .and_then(|v| v.as_str())
        .unwrap_or("mock");
    let min_confidence = config
        .pipeline
        .scanned_tables
        .get("min_confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5) as f32;
    let create_placeholder = config
        .pipeline
        .scanned_tables
        .get("create_placeholder_tables")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let detector: Box<dyn ScannedTableDetector> = match backend.to_ascii_lowercase().as_str() {
        "fixture" => Box::new(FixtureScannedTableDetector),
        "disabled" => Box::new(DisabledScannedTableDetector),
        _ => Box::new(MockScannedTableDetector),
    };
    let recognizer: Box<dyn TableStructureRecognizer> = match backend.to_ascii_lowercase().as_str() {
        "fixture" => Box::new(FixtureTableStructureRecognizer),
        "disabled" => Box::new(DisabledTableStructureRecognizer),
        _ => Box::new(MockTableStructureRecognizer),
    };

    let mut produced_regions = 0usize;
    for page_idx in 0..model.pages.len() {
        let page = &model.pages[page_idx];
        let input = TableDetectionInput {
            document_id: model.document_id.clone(),
            page_number: page.page_number as usize,
            page_image_path: resolve_page_image_path(model, page_idx, input_path),
            page_width: page.width.unwrap_or(1000.0),
            page_height: page.height.unwrap_or(1400.0),
        };

        let mut ex_ctx = ExtractionContext::default();
        let regions = match run_async(detector.detect_tables(input, &mut ex_ctx)) {
            Ok(v) => v,
            Err(err) => {
                model.warnings.push(Diagnostic {
                    code: "SCANNED_TABLE_DETECTION_FAILED".to_string(),
                    severity: "warning".to_string(),
                    scope: "page".to_string(),
                    page_number: Some(page.page_number),
                    element_id: None,
                    message: err.message,
                    recoverable: true,
                    extra: Default::default(),
                });
                Vec::new()
            }
        };

        for region in regions {
            if region.confidence < min_confidence {
                continue;
            }
            let page_mut = &mut model.pages[page_idx];
            let as_layout = LayoutRegion {
                region_id: region.region_id.clone(),
                page_number: region.page_number,
                region_type: crate::layout::LayoutRegionType::Table,
                bbox: region.bbox,
                polygon: None,
                confidence: region.confidence,
                reading_order: None,
                source: LayoutSource::Mock,
            };
            if has_region_duplicate(page_mut, &as_layout) {
                continue;
            }

            let element = if create_placeholder {
                create_scanned_table_placeholder(
                    region.page_number,
                    &region.region_id,
                    region.bbox,
                    region.confidence,
                    &region.source,
                )
            } else {
                match run_async(recognizer.recognize_structure(
                    TableStructureInput {
                        document_id: model.document_id.clone(),
                        table_region: region.clone(),
                    },
                    &mut ex_ctx,
                )) {
                    Ok(el) => el,
                    Err(_) => create_scanned_table_placeholder(
                        region.page_number,
                        &region.region_id,
                        region.bbox,
                        region.confidence,
                        &region.source,
                    ),
                }
            };
            page_mut.elements.push(element);
            produced_regions += 1;
        }
    }

    model.processing.stages.push(ProcessingStage {
        name: "scanned_table_detection".to_string(),
        status: StageStatus::Ok,
        tool: format!("{}_scanned_table_detector", backend),
        duration_ms: Some(1),
        metadata: json!({"regions": produced_regions}),
    });
    model.processing.stages.push(ProcessingStage {
        name: "table_structure_recognition".to_string(),
        status: StageStatus::Ok,
        tool: format!("{}_table_structure_recognizer", backend),
        duration_ms: Some(1),
        metadata: json!({"regions": produced_regions, "placeholder": create_placeholder}),
    });
}

fn apply_formula_pipeline(input_path: &Path, config: &PipelineConfig, model: &mut DocumentModel) {
    let enabled = cli_or_config_bool(
        pipeline_cli_overrides().and_then(|o| o.detect_formulas),
        config.pipeline.formulas.get("enabled"),
        true,
    );
    if !enabled {
        model.processing.stages.push(ProcessingStage {
            name: "formula_detection".to_string(),
            status: StageStatus::Skipped,
            tool: "formula_detector".to_string(),
            duration_ms: Some(0),
            metadata: json!({"reason": "disabled"}),
        });
        return;
    }

    let backend = config
        .pipeline
        .formulas
        .get("backend")
        .and_then(|v| v.as_str())
        .unwrap_or("mock");
    let create_placeholder = config
        .pipeline
        .formulas
        .get("create_placeholder_formulas")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let recognize_scanned = config
        .pipeline
        .formulas
        .get("recognize_scanned")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let detector: Box<dyn FormulaDetector> = match backend.to_ascii_lowercase().as_str() {
        "fixture" => Box::new(FixtureFormulaDetector),
        "disabled" => Box::new(DisabledFormulaDetector),
        _ => Box::new(MockFormulaDetector),
    };
    let recognizer: Box<dyn FormulaRecognizer> = match backend.to_ascii_lowercase().as_str() {
        "fixture" => Box::new(FixtureFormulaRecognizer),
        "disabled" => Box::new(DisabledFormulaRecognizer),
        _ => Box::new(MockFormulaRecognizer),
    };

    let mut produced_regions = 0usize;
    for page_idx in 0..model.pages.len() {
        let page = &model.pages[page_idx];
        let input = FormulaDetectionInput {
            document_id: model.document_id.clone(),
            page_number: page.page_number as usize,
            page_image_path: resolve_page_image_path(model, page_idx, input_path),
            page_width: page.width.unwrap_or(1000.0),
            page_height: page.height.unwrap_or(1400.0),
        };
        let mut ex_ctx = ExtractionContext::default();
        let regions = match run_async(detector.detect_formulas(input, &mut ex_ctx)) {
            Ok(v) => v,
            Err(err) => {
                model.warnings.push(Diagnostic {
                    code: "FORMULA_DETECTION_FAILED".to_string(),
                    severity: "warning".to_string(),
                    scope: "page".to_string(),
                    page_number: Some(page.page_number),
                    element_id: None,
                    message: err.message,
                    recoverable: true,
                    extra: Default::default(),
                });
                Vec::new()
            }
        };

        for region in regions {
            let page_mut = &mut model.pages[page_idx];
            let as_layout = LayoutRegion {
                region_id: region.region_id.clone(),
                page_number: region.page_number,
                region_type: crate::layout::LayoutRegionType::Formula,
                bbox: region.bbox,
                polygon: None,
                confidence: region.confidence,
                reading_order: None,
                source: LayoutSource::Mock,
            };
            if has_region_duplicate(page_mut, &as_layout) {
                continue;
            }

            let element = if create_placeholder || !recognize_scanned {
                create_formula_placeholder(
                    region.page_number,
                    &region.region_id,
                    region.bbox,
                    region.confidence,
                    &region.source,
                )
            } else {
                match run_async(recognizer.recognize_formula(
                    FormulaRecognitionInput {
                        document_id: model.document_id.clone(),
                        formula_region: region.clone(),
                    },
                    &mut ex_ctx,
                )) {
                    Ok(el) => el,
                    Err(_) => create_formula_placeholder(
                        region.page_number,
                        &region.region_id,
                        region.bbox,
                        region.confidence,
                        &region.source,
                    ),
                }
            };
            page_mut.elements.push(element);
            produced_regions += 1;
        }
    }

    model.processing.stages.push(ProcessingStage {
        name: "formula_detection".to_string(),
        status: StageStatus::Ok,
        tool: format!("{}_formula_detector", backend),
        duration_ms: Some(1),
        metadata: json!({"regions": produced_regions}),
    });
    model.processing.stages.push(ProcessingStage {
        name: "formula_recognition".to_string(),
        status: StageStatus::Ok,
        tool: format!("{}_formula_recognizer", backend),
        duration_ms: Some(1),
        metadata: json!({"regions": produced_regions, "placeholder": create_placeholder || !recognize_scanned}),
    });
}

fn resolve_page_image_path(model: &DocumentModel, page_idx: usize, input_path: &Path) -> Option<std::path::PathBuf> {
    let page = model.pages.get(page_idx)?;
    if page.page_type == crate::model::PageType::Image {
        return Some(input_path.to_path_buf());
    }

    let asset_id = page.page_image_asset_id.as_ref()?;
    let asset = model.assets.iter().find(|a| &a.asset_id == asset_id)?;
    Some(output_root_dir().join(&model.document_id).join(&asset.path))
}

fn has_region_duplicate(page: &crate::model::Page, region: &LayoutRegion) -> bool {
    let region_bbox = region.bbox;
    page.elements.iter().any(|element| {
        let same_type = matches!(
            (&region.region_type, &element.element_type),
            (crate::layout::LayoutRegionType::Table, ElementType::Table)
                | (crate::layout::LayoutRegionType::Formula, ElementType::Formula)
                | (crate::layout::LayoutRegionType::Figure, ElementType::Image)
                | (crate::layout::LayoutRegionType::Watermark, ElementType::Watermark)
        );
        if !same_type {
            return false;
        }
        let Some(bbox) = element.bbox else {
            return false;
        };
        let element_bbox = BBox::from_array(bbox);
        bbox_iou(&element_bbox, &region_bbox) >= 0.5
    })
}

fn write_layout_debug_artifacts_if_needed(
    _input_path: &Path,
    config: &PipelineConfig,
    model: &mut DocumentModel,
    layout_regions: &[LayoutRegion],
    save_debug_artifacts: bool,
) {
    let write_layout_json = config
        .pipeline
        .debug
        .get("write_layout_json")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || save_debug_artifacts
        || pipeline_cli_overrides()
            .and_then(|o| o.debug_layout)
            .unwrap_or(false);

    if !write_layout_json {
        return;
    }

    let store = LocalAssetStore::new(output_root_dir());
    let grouped = layout_regions
        .iter()
        .fold(std::collections::HashMap::<u32, Vec<LayoutRegion>>::new(), |mut acc, region| {
            acc.entry(region.page_number as u32)
                .or_default()
                .push(region.clone());
            acc
        });

    for page in &model.pages {
        let page_regions = grouped.get(&page.page_number).cloned().unwrap_or_default();
        let layout_json = crate::layout::debug::regions_to_json(&page_regions);
        let ro_json = crate::layout::debug::page_reading_order_snapshot(page);

        let layout_name = format!("page_{}_layout_regions.json", page.page_number);
        match write_debug_json_asset(&store, &model.document_id, &layout_name, &layout_json) {
            Ok(mut asset) => {
                asset.page_number = Some(page.page_number);
                model.assets.push(asset);
            }
            Err(err) => {
                model.warnings.push(Diagnostic {
                    code: "DEBUG_ARTIFACT_WRITE_FAILED".to_string(),
                    severity: "warning".to_string(),
                    scope: "page".to_string(),
                    page_number: Some(page.page_number),
                    element_id: None,
                    message: format!("Не удалось сохранить layout debug JSON: {err}"),
                    recoverable: true,
                    extra: Default::default(),
                });
            }
        }

        let ro_name = format!("page_{}_reading_order.json", page.page_number);
        match write_debug_json_asset(&store, &model.document_id, &ro_name, &ro_json) {
            Ok(mut asset) => {
                asset.page_number = Some(page.page_number);
                model.assets.push(asset);
            }
            Err(err) => {
                model.warnings.push(Diagnostic {
                    code: "DEBUG_ARTIFACT_WRITE_FAILED".to_string(),
                    severity: "warning".to_string(),
                    scope: "page".to_string(),
                    page_number: Some(page.page_number),
                    element_id: None,
                    message: format!("Не удалось сохранить reading-order debug JSON: {err}"),
                    recoverable: true,
                    extra: Default::default(),
                });
            }
        }
    }

    model.processing.stages.push(ProcessingStage {
        name: "layout_debug_artifacts".to_string(),
        status: StageStatus::Ok,
        tool: "debug_artifacts_writer".to_string(),
        duration_ms: Some(1),
        metadata: json!({"pages": model.pages.len()}),
    });
}
