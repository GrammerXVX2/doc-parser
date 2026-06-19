use std::path::Path;

use anyhow::Context;
use futures::executor::block_on;
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
            match block_on(detector.detect_layout(input, &mut ex_ctx)) {
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
        let regions = match block_on(detector.detect_tables(input, &mut ex_ctx)) {
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
                match block_on(recognizer.recognize_structure(
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
        let regions = match block_on(detector.detect_formulas(input, &mut ex_ctx)) {
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
                match block_on(recognizer.recognize_formula(
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
