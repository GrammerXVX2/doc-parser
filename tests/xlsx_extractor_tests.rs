use std::path::PathBuf;

use document_parser::classifier::classify_file;
use document_parser::config::{load_format_routing_config, load_pipeline_config};
use document_parser::extractors::xlsx::XlsxExtractor;
use document_parser::pipeline::{PipelineContext, run_pipeline};
use document_parser::router::Extractor;
use document_parser::validation::{ValidationSeverity, validate_document_model};

fn pipeline_context() -> PipelineContext {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pipeline = load_pipeline_config(&root.join("configs/pipeline.config.jsonc"))
        .expect("pipeline config should load");
    let routing = load_format_routing_config(&root.join("configs/format_routing.config.jsonc"))
        .expect("routing config should load");
    PipelineContext::new(pipeline, routing)
}

#[test]
fn xlsx_extractor_builds_sheet_tables_and_formula_metadata() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("testdata/office/sample_finance.xlsx");
    let classification = classify_file(&path).expect("classification");

    let model = XlsxExtractor
        .extract(&path, &classification)
        .expect("xlsx extraction");

    assert!(matches!(model.pages[0].page_type, document_parser::model::PageType::Sheet));

    let sheet_meta = model.pages[0]
        .extra
        .get("sheet")
        .and_then(|v| v.as_object())
        .expect("sheet metadata");
    assert_eq!(sheet_meta.get("name").and_then(|v| v.as_str()), Some("Лист1"));

    let table = model.pages[0]
        .elements
        .iter()
        .find(|e| matches!(e.element_type, document_parser::model::ElementType::Table))
        .expect("table element");
    assert!(table.extra.get("cells").is_some());

    let cells = table
        .extra
        .get("cells")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(!cells.is_empty());
    assert!(cells[0].get("formula").is_some());

    assert!(model.pages[0].text.contains("Метрика"));
    assert!(model.assets.iter().any(|a| a.asset_type == "embedded_image"));

    let issues = validate_document_model(&model);
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.severity, ValidationSeverity::Fatal)),
        "fatal validation issues: {:?}",
        issues
    );
}

#[test]
fn xlsx_pipeline_creates_table_chunks() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("testdata/office/sample_finance.xlsx");
    let (_, model) = run_pipeline(&path, &pipeline_context()).expect("pipeline");

    assert!(model.chunks.iter().any(|c| {
        c.metadata
            .get("contains_table")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }));
}
