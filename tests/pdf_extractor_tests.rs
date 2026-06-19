use std::collections::HashSet;
use std::path::PathBuf;

use document_parser::config::{load_format_routing_config, load_pipeline_config};
use document_parser::model::{DocumentFormat, ElementType, ProcessingStatus};
use document_parser::pipeline::{PipelineContext, run_pipeline};

fn pipeline_context() -> PipelineContext {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pipeline = load_pipeline_config(&root.join("configs/pipeline.config.jsonc"))
        .expect("pipeline config should load");
    let routing = load_format_routing_config(&root.join("configs/format_routing.config.jsonc"))
        .expect("routing config should load");
    PipelineContext::new(pipeline, routing)
}

#[test]
fn digital_pdf_has_native_text_elements() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input = root.join("testdata/pdf/digital.pdf");

    let (_, model) = run_pipeline(&input, &pipeline_context()).expect("pdf pipeline should succeed");
    assert_eq!(model.document_profile.format, DocumentFormat::Pdf);
    assert!(!model.pages.is_empty());

    let has_native_text_element = model.pages.iter().any(|p| {
        p.elements
            .iter()
            .any(|e| matches!(e.element_type, ElementType::Text | ElementType::Heading | ElementType::Paragraph))
    });
    assert!(has_native_text_element, "digital pdf should produce native text elements");

    assert!(
        matches!(model.processing.status, ProcessingStatus::Ok | ProcessingStatus::Partial),
        "digital pdf status should be ok or partial"
    );
}

#[test]
fn scanned_pdf_runs_render_and_mock_ocr() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input = root.join("testdata/pdf/scanned.pdf");

    let (_, model) = run_pipeline(&input, &pipeline_context()).expect("pdf pipeline should succeed");
    assert!(!model.pages.is_empty());

    let page = &model.pages[0];
    assert!(page.page_profile.has_ocr_required_regions);
    assert!(page.page_image_asset_id.is_some(), "scanned page should have rendered image asset");
    assert!(
        page.elements
            .iter()
            .any(|e| matches!(e.element_type, ElementType::TextOcr)),
        "scanned page should contain OCR elements"
    );
}

#[test]
fn hybrid_pdf_merges_without_duplicate_text_lines() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input = root.join("testdata/pdf/hybrid.pdf");

    let (_, model) = run_pipeline(&input, &pipeline_context()).expect("pdf pipeline should succeed");

    let has_native = model.pages.iter().any(|p| {
        p.elements
            .iter()
            .any(|e| matches!(e.element_type, ElementType::Text | ElementType::Heading | ElementType::Paragraph))
    });
    let has_ocr = model.pages.iter().any(|p| {
        p.elements
            .iter()
            .any(|e| matches!(e.element_type, ElementType::TextOcr))
    });

    assert!(has_native || has_ocr, "hybrid should produce native and/or OCR content");

    let lines = model
        .pages
        .iter()
        .flat_map(|p| p.text.lines().map(|l| l.trim().to_string()))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>();
    let unique = lines.iter().cloned().collect::<HashSet<_>>();
    assert_eq!(lines.len(), unique.len(), "merged text should not contain duplicate lines");
}
