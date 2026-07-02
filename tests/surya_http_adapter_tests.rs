use document_parser::models::config::ModelBackendConfig;
use document_parser::models::layout::SuryaLayoutHttpBackend;
use document_parser::models::tables::SuryaTableHttpBackend;
use document_parser::models::backends::traits::ModelBackend;

#[tokio::test]
async fn surya_layout_health_unavailable() {
    let cfg = ModelBackendConfig {
        kind: "layout".to_string(),
        enabled: true,
        backend_type: "http".to_string(),
        required: false,
        url: Some("http://127.0.0.1:38121".to_string()),
        health_path: Some("/healthz".to_string()),
        layout_path: Some("/v1/layout".to_string()),
        timeout_sec: Some(1),
        ..Default::default()
    };

    let backend = SuryaLayoutHttpBackend::new(cfg);
    let health = backend.health_check().await;
    assert!(!health.available);
}

#[tokio::test]
async fn surya_table_health_unavailable() {
    let cfg = ModelBackendConfig {
        kind: "table_structure".to_string(),
        enabled: true,
        backend_type: "http".to_string(),
        required: false,
        url: Some("http://127.0.0.1:38122".to_string()),
        health_path: Some("/healthz".to_string()),
        table_path: Some("/v1/tables".to_string()),
        timeout_sec: Some(1),
        ..Default::default()
    };

    let backend = SuryaTableHttpBackend::new(cfg);
    let health = backend.health_check().await;
    assert!(!health.available);
}
