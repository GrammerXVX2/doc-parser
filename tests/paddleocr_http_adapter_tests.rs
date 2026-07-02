use document_parser::converters::traits::ExtractionContext;
use document_parser::models::backends::traits::{ExtendedOcrBackend, ExtendedOcrInput};
use document_parser::models::config::ModelBackendConfig;
use document_parser::models::ocr::PaddleOcrV6HttpBackend;

#[tokio::test]
async fn paddleocr_adapter_fallback_when_service_unavailable() {
    let cfg = ModelBackendConfig {
        kind: "ocr".to_string(),
        enabled: true,
        backend_type: "http".to_string(),
        required: false,
        url: Some("http://127.0.0.1:38111".to_string()),
        ocr_path: Some("/v1/ocr".to_string()),
        health_path: Some("/healthz".to_string()),
        timeout_sec: Some(1),
        ..Default::default()
    };

    let backend = PaddleOcrV6HttpBackend::new(cfg);
    let mut ctx = ExtractionContext::default();
    let input = ExtendedOcrInput {
        document_id: "doc1".to_string(),
        page_number: 1,
        image_path: Some("/tmp/nonexistent.png".to_string()),
        languages: vec!["ru".to_string(), "en".to_string()],
    };

    let out = backend.run_ocr(input, &mut ctx).await;
    assert!(out.is_err());
}
