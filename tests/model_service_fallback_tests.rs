use std::path::PathBuf;

use document_parser::config::{load_format_routing_config, load_pipeline_config};
use document_parser::pipeline::{PipelineContext, run_pipeline};

#[test]
fn pipeline_uses_fallback_when_model_services_unavailable() {
    let pipeline = load_pipeline_config(PathBuf::from("configs/pipeline.config.jsonc").as_path()).unwrap();
    let routing = load_format_routing_config(PathBuf::from("configs/format_routing.config.jsonc").as_path()).unwrap();
    let context = PipelineContext::new(pipeline, routing);

    let input = PathBuf::from("testdata/ru/sample_ru.html");
    let (_, model) = run_pipeline(&input, &context).unwrap();

    assert!(model
        .warnings
        .iter()
        .any(|w| w.code == "MODEL_BACKEND_FALLBACK_USED" || w.code.ends_with("SERVICE_UNAVAILABLE")));

    let outputs = model.extra.get("model_outputs").expect("model_outputs");
    assert!(outputs.get("ocr").is_some());
    assert!(outputs.get("layout").is_some());
    assert!(outputs.get("structured_document_parse").is_some());
}
