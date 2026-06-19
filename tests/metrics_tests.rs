use std::path::PathBuf;

use document_parser::api::{build_app, build_state};
use document_parser::config::profiles::ServiceProfile;
use tower::ServiceExt;

#[tokio::test]
async fn metrics_endpoint_returns_text() {
    let profile = ServiceProfile::from_path(&PathBuf::from("configs/profiles/api.jsonc")).expect("profile");
    let state = build_state(profile);
    state.metrics.inc("api_requests_total").await;
    let app = build_app(state);

    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/metrics")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let headers = resp.headers();
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert!(content_type.contains("text/plain"));
}
