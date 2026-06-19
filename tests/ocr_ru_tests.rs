use std::path::PathBuf;

use document_parser::config::{load_format_routing_config, load_pipeline_config};
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
fn russian_ocr_text_normalization_and_low_confidence_warning() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input = root.join("testdata/images/scan.png");

    let (_, model) = run_pipeline(&input, &pipeline_context()).expect("pipeline should succeed");

    let ocr_elements = model.pages[0]
        .elements
        .iter()
        .filter(|e| matches!(e.element_type, document_parser::model::ElementType::TextOcr))
        .collect::<Vec<_>>();
    assert!(!ocr_elements.is_empty());

    assert!(ocr_elements.iter().any(|e| {
        e.extra
            .get("language")
            .and_then(|v| v.as_str())
            .map(|v| v == "ru")
            .unwrap_or(false)
    }));

    let low_conf_warning_exists = ocr_elements.iter().any(|e| {
        e.warnings
            .iter()
            .any(|w| w.code == "LOW_OCR_CONFIDENCE" || w.message.contains("Низкая уверенность OCR"))
    });
    assert!(low_conf_warning_exists);
}
