use std::path::PathBuf;

use document_parser::api::{build_app, build_state};
use document_parser::config::profiles::ServiceProfile;
use tower::ServiceExt;

fn set_test_env(key: &str, value: &str) {
    // Tests here run against a local process context.
    unsafe {
        std::env::set_var(key, value);
    }
}

fn remove_test_env(key: &str) {
    // Tests here run against a local process context.
    unsafe {
        std::env::remove_var(key);
    }
}

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
async fn auth_disabled_upload_works_without_token() {
    let mut profile =
        ServiceProfile::from_path(&PathBuf::from("configs/profiles/dev_team.jsonc")).unwrap();
    profile.auth.enabled = false;

    let app = build_app(build_state(profile));
    let boundary = "x-boundary";

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/documents")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .body(axum::body::Body::from(multipart_body(
            boundary,
            "sample.md",
            b"# test",
        )))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn auth_enabled_without_token_rejected() {
    remove_test_env("DOC_PARSER_TEST_TOKEN_MISSING");

    let mut profile =
        ServiceProfile::from_path(&PathBuf::from("configs/profiles/dev_team.jsonc")).unwrap();
    profile.auth.enabled = true;
    profile.auth.dev_token_env = "DOC_PARSER_TEST_TOKEN_MISSING".to_string();

    let app = build_app(build_state(profile));
    let boundary = "x-boundary";

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/documents")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .body(axum::body::Body::from(multipart_body(
            boundary,
            "sample.md",
            b"# test",
        )))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn auth_enabled_wrong_token_rejected() {
    set_test_env("DOC_PARSER_TEST_TOKEN_WRONG", "expected-token");

    let mut profile =
        ServiceProfile::from_path(&PathBuf::from("configs/profiles/dev_team.jsonc")).unwrap();
    profile.auth.enabled = true;
    profile.auth.dev_token_env = "DOC_PARSER_TEST_TOKEN_WRONG".to_string();

    let app = build_app(build_state(profile));
    let boundary = "x-boundary";

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/documents")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .header("Authorization", "Bearer definitely_wrong")
        .body(axum::body::Body::from(multipart_body(
            boundary,
            "sample.md",
            b"# test",
        )))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn auth_enabled_correct_token_accepts_upload() {
    let expected = "expected-token";
    set_test_env("DOC_PARSER_TEST_TOKEN_OK", expected);

    let mut profile =
        ServiceProfile::from_path(&PathBuf::from("configs/profiles/dev_team.jsonc")).unwrap();
    profile.auth.enabled = true;
    profile.auth.dev_token_env = "DOC_PARSER_TEST_TOKEN_OK".to_string();

    let app = build_app(build_state(profile));
    let boundary = "x-boundary";

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/documents")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .header("Authorization", format!("Bearer {}", expected))
        .body(axum::body::Body::from(multipart_body(
            boundary,
            "sample.md",
            b"# test",
        )))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
}
