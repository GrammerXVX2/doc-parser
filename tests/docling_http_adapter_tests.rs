use document_parser::converters::traits::ExtractionContext;
use document_parser::models::backends::traits::{
    ModelBackend, StructuredDocumentParserBackend, StructuredParseInput,
};
use document_parser::models::config::ModelBackendConfig;
use document_parser::models::structured::DoclingStructuredParseHttpBackend;

#[tokio::test]
async fn docling_health_unavailable() {
    let cfg = ModelBackendConfig {
        kind: "structured_document_parse".to_string(),
        enabled: true,
        backend_type: "http".to_string(),
        required: false,
        url: Some("http://127.0.0.1:38131".to_string()),
        health_path: Some("/healthz".to_string()),
        parse_path: Some("/v1/parse".to_string()),
        timeout_sec: Some(1),
        ..Default::default()
    };

    let backend = DoclingStructuredParseHttpBackend::new(cfg);
    let health = backend.health_check().await;
    assert!(!health.available);
}

#[tokio::test]
async fn docling_parse_fails_gracefully_when_unavailable() {
    let cfg = ModelBackendConfig {
        kind: "structured_document_parse".to_string(),
        enabled: true,
        backend_type: "http".to_string(),
        required: false,
        url: Some("http://127.0.0.1:38132".to_string()),
        health_path: Some("/healthz".to_string()),
        parse_path: Some("/v1/parse".to_string()),
        timeout_sec: Some(1),
        ..Default::default()
    };

    let backend = DoclingStructuredParseHttpBackend::new(cfg);
    let mut ctx = ExtractionContext::default();
    let input = StructuredParseInput {
        document_id: "doc1".to_string(),
        input_path: "/tmp/a.pdf".to_string(),
    };

    let out = backend.parse_document_structured(input, &mut ctx).await;
    assert!(out.is_err());
}
