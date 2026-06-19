use std::path::PathBuf;

use document_parser::config::{load_format_routing_config, load_pipeline_config};
use document_parser::model::{ElementType, PageType};
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
fn image_input_creates_page_asset_and_mock_ocr_elements() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input = root.join("testdata/images/scan.png");

    let (_, model) = run_pipeline(&input, &pipeline_context()).expect("image pipeline should succeed");

    assert_eq!(model.pages.len(), 1);
    let page = &model.pages[0];
    assert_eq!(page.page_type, PageType::Image);
    assert!(page.page_image_asset_id.is_some());
    assert!(!model.assets.is_empty());

    assert!(
        page.elements
            .iter()
            .any(|e| matches!(e.element_type, ElementType::TextOcr)),
        "image extractor should emit OCR text elements"
    );
    assert!(page.text.contains("Invoice number"));
}
