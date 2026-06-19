use std::collections::HashSet;
use std::path::PathBuf;

use document_parser::config::{load_format_routing_config, load_pipeline_config};
use document_parser::pipeline::{PipelineContext, run_pipeline};
use document_parser::validation::{ValidationSeverity, validate_document_model};

fn pipeline_context() -> PipelineContext {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pipeline = load_pipeline_config(&root.join("configs/pipeline.config.jsonc"))
        .expect("pipeline config should load");
    let routing = load_format_routing_config(&root.join("configs/format_routing.config.jsonc"))
        .expect("routing config should load");
    PipelineContext::new(pipeline, routing)
}

fn assert_asset_refs_exist(model: &document_parser::model::DocumentModel) {
    let assets = model
        .assets
        .iter()
        .map(|a| a.asset_id.clone())
        .collect::<HashSet<_>>();

    for page in &model.pages {
        for element in &page.elements {
            if let Some(asset_id) = element.extra.get("asset_id").and_then(|v| v.as_str()) {
                assert!(assets.contains(asset_id), "missing asset ref: {asset_id}");
            }
        }
    }
}

#[test]
fn docx_and_xlsx_embedded_images_saved_and_referenced() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let (_, docx_model) = run_pipeline(&root.join("testdata/office/sample_ru.docx"), &pipeline_context())
        .expect("docx pipeline");
    assert!(!docx_model.assets.is_empty());
    assert_asset_refs_exist(&docx_model);

    let (_, xlsx_model) = run_pipeline(&root.join("testdata/office/sample_finance.xlsx"), &pipeline_context())
        .expect("xlsx pipeline");
    assert!(!xlsx_model.assets.is_empty());
    assert_asset_refs_exist(&xlsx_model);

    let issues = validate_document_model(&docx_model);
    assert!(!issues.iter().any(|i| matches!(i.severity, ValidationSeverity::Fatal)));

    let issues = validate_document_model(&xlsx_model);
    assert!(!issues.iter().any(|i| matches!(i.severity, ValidationSeverity::Fatal)));
}
