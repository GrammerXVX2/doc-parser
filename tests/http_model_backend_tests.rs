use std::time::Duration;

use document_parser::models::backends::http::HttpModelBackendClient;
use serde_json::json;

#[tokio::test]
async fn health_check_handles_unavailable_service() {
    let client = HttpModelBackendClient::new(
        "test_backend",
        "http://127.0.0.1:38101",
        Duration::from_millis(300),
    );

    let health = client.health_check("/healthz").await;
    assert!(!health.available);
}

#[tokio::test]
async fn post_json_handles_timeout() {
    let client = HttpModelBackendClient::new(
        "test_backend",
        "http://10.255.255.1:38102",
        Duration::from_millis(100),
    );

    let result: anyhow::Result<serde_json::Value> = client.post_json("/v1/ocr", &json!({})).await;
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(err.contains("MODEL_BACKEND_TIMEOUT") || err.contains("MODEL_BACKEND_HTTP_ERROR"));
}

#[tokio::test]
async fn post_json_handles_invalid_response() {
    let client = HttpModelBackendClient::new(
        "test_backend",
        "http://127.0.0.1:38103",
        Duration::from_millis(300),
    );

    let result: anyhow::Result<serde_json::Value> = client.post_json("/v1/ocr", &json!({})).await;
    assert!(result.is_err());
}
