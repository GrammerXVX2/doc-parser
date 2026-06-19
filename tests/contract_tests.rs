use std::path::PathBuf;

use document_parser::config::{FormatRoutingConfig, PipelineConfig, load_jsonc_file};
use document_parser::model::DocumentModel;
use document_parser::validation::{ValidationSeverity, validate_document_model};

#[test]
fn examples_deserialize_into_document_model() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let examples = [
        root.join("examples/html_document.example.json"),
        root.join("examples/hybrid_pdf_document.example.jsonc"),
        root.join("examples/xlsx_document.example.jsonc"),
    ];

    for example in examples {
        let value: serde_json::Value = load_jsonc_file(&example)
            .unwrap_or_else(|e| panic!("failed to load example {}: {e:#}", example.display()));

        let model: DocumentModel = serde_json::from_value(value)
            .unwrap_or_else(|e| panic!("failed to parse model {}: {e:#}", example.display()));

        let serialized = serde_json::to_value(&model)
            .unwrap_or_else(|e| panic!("failed to serialize model {}: {e:#}", example.display()));
        let _roundtrip: DocumentModel = serde_json::from_value(serialized)
            .unwrap_or_else(|e| panic!("failed to roundtrip model {}: {e:#}", example.display()));

        assert_eq!(model.schema_version, "1.0.0");
        assert!(!model.document_id.is_empty());
        assert!(!model.pages.is_empty());
        assert!(!model.processing.stages.is_empty());

        let issues = validate_document_model(&model);
        assert!(
            !issues
                .iter()
                .any(|i| matches!(i.severity, ValidationSeverity::Fatal)),
            "expected no fatal issues for {}, got {:?}",
            example.display(),
            issues
        );
    }
}

#[test]
fn configs_are_valid_jsonc() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pipeline_path = root.join("configs/pipeline.config.jsonc");
    let routing_path = root.join("configs/format_routing.config.jsonc");

    let pipeline: PipelineConfig = load_jsonc_file(&pipeline_path)
        .unwrap_or_else(|e| panic!("invalid pipeline config: {e:#}"));
    let routing: FormatRoutingConfig = load_jsonc_file(&routing_path)
        .unwrap_or_else(|e| panic!("invalid routing config: {e:#}"));

    assert_eq!(pipeline.pipeline.version, "1.0.0");
    assert!(routing.routing.contains_key("html"));
    assert!(routing.routing.contains_key("md"));
    assert!(routing.routing.contains_key("txt"));
}

#[test]
fn schema_files_are_valid_jsonc() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let paths = [
        root.join("schemas/document_model.schema.jsonc"),
        root.join("schemas/element_types.schema.jsonc"),
    ];

    for path in paths {
        let value: serde_json::Value = load_jsonc_file(&path)
            .unwrap_or_else(|e| panic!("invalid schema {}: {e:#}", path.display()));
        assert!(value.is_object());
    }
}
