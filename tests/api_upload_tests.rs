use std::path::PathBuf;

use document_parser::api::{build_app, build_state};
use document_parser::config::profiles::ServiceProfile;
use tower::ServiceExt;

fn multipart_body(boundary: &str, filename: &str, bytes: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n",
            filename
        )
        .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());
    body
}

#[tokio::test]
async fn upload_valid_file_queued() {
    let profile = ServiceProfile::from_path(&PathBuf::from("configs/profiles/api.jsonc")).expect("profile");
    let app = build_app(build_state(profile));
    let boundary = "x-boundary";

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/documents")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .body(axum::body::Body::from(multipart_body(boundary, "sample.md", b"# test")))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn upload_empty_file_error() {
    let profile = ServiceProfile::from_path(&PathBuf::from("configs/profiles/api.jsonc")).expect("profile");
    let app = build_app(build_state(profile));
    let boundary = "x-boundary";

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/documents")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .body(axum::body::Body::from(multipart_body(boundary, "sample.md", b"")))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn upload_unsupported_extension_error() {
    let profile = ServiceProfile::from_path(&PathBuf::from("configs/profiles/api.jsonc")).expect("profile");
    let app = build_app(build_state(profile));
    let boundary = "x-boundary";

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/documents")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .body(axum::body::Body::from(multipart_body(boundary, "bad.exe", b"abc")))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn upload_too_large_error() {
    let mut profile = ServiceProfile::from_path(&PathBuf::from("configs/profiles/api.jsonc")).expect("profile");
    profile.security.max_file_size_mb = 1;
    let app = build_app(build_state(profile));
    let boundary = "x-boundary";
    let large = vec![b'a'; 2 * 1024 * 1024];

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/documents")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .body(axum::body::Body::from(multipart_body(boundary, "sample.md", &large)))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::PAYLOAD_TOO_LARGE);
}
