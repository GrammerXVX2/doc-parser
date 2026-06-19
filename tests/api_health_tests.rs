use std::path::PathBuf;

use document_parser::api::{build_app, build_state};
use document_parser::config::profiles::ServiceProfile;
use tower::ServiceExt;

#[tokio::test]
async fn health_and_ready_return_200() {
    let profile = ServiceProfile::from_path(&PathBuf::from("configs/profiles/api.jsonc")).expect("profile");
    let app = build_app(build_state(profile));

    let health = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/healthz")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(health.status(), axum::http::StatusCode::OK);

    let ready = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/readyz")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ready.status(), axum::http::StatusCode::OK);
}
