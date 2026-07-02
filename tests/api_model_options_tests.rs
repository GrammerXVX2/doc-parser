use std::path::PathBuf;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use document_parser::api::{build_app, build_state};
use document_parser::config::profiles::ServiceProfile;
use serde_json::Value;
use tower::ServiceExt;

fn multipart_with_options(boundary: &str) -> Vec<u8> {
    let mut body = Vec::new();

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"sample.txt\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: text/plain\r\n\r\n");
    body.extend_from_slice("Договор. ИНН 7701234567".as_bytes());
    body.extend_from_slice(format!("\r\n--{}\r\n", boundary).as_bytes());

    body.extend_from_slice(b"Content-Disposition: form-data; name=\"model_profile\"\r\n\r\nlegal_fast");
    body.extend_from_slice(format!("\r\n--{}\r\n", boundary).as_bytes());

    body.extend_from_slice(b"Content-Disposition: form-data; name=\"legal_extract\"\r\n\r\ntrue");
    body.extend_from_slice(format!("\r\n--{}\r\n", boundary).as_bytes());

    body.extend_from_slice(b"Content-Disposition: form-data; name=\"enable_slow_path\"\r\n\r\ntrue");
    body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

    body
}

#[tokio::test]
async fn multipart_model_options_reach_processing_options() {
    let mut profile = ServiceProfile::from_path(&PathBuf::from("configs/profiles/api.jsonc")).unwrap();
    profile.auth.enabled = false;

    let state = build_state(profile);
    let app = build_app(state.clone());

    let boundary = "opts-boundary";
    let req = Request::builder()
        .method("POST")
        .uri("/v1/documents")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .body(Body::from(multipart_with_options(boundary)))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    let job_id = payload.get("job_id").and_then(Value::as_str).unwrap();

    let job = state.registry.get(job_id).await.unwrap().unwrap();
    assert_eq!(job.options.model_profile.as_deref(), Some("legal_fast"));
    assert_eq!(job.options.legal_extract, Some(true));
    assert_eq!(job.options.enable_slow_path, Some(true));
}
