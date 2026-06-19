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
fn pdf_pipeline_contains_layout_stages_and_reading_order() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input = root.join("testdata/pdf/digital.pdf");

    let (_, model) = run_pipeline(&input, &pipeline_context()).expect("pipeline should run");

    let stage_names = model
        .processing
        .stages
        .iter()
        .map(|s| s.name.as_str())
        .collect::<Vec<_>>();

    assert!(stage_names.contains(&"layout_aware_reading_order"));
    assert!(stage_names.contains(&"layout_detection") || stage_names.contains(&"layout_debug_artifacts"));

    assert!(model.pages.iter().all(|p| {
        p.elements
            .iter()
            .enumerate()
            .all(|(i, e)| e.reading_order == Some((i + 1) as u32))
    }));
}
