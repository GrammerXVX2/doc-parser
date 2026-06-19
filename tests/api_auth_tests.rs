use std::path::Path;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use document_parser::api::{build_app, build_state};
use document_parser::config::profiles::ServiceProfile;
use tower::ServiceExt;

fn multipart_upload_request(token: Option<&str>) -> Request<Body> {
    let boundary = "x-boundary";
    let mut body = Vec::new();

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"sample.md\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: text/markdown\r\n\r\n");
    body.extend_from_slice(b"# test\ntext");
    body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

    let mut builder = Request::builder()
        .method("POST")
        .uri("/v1/documents")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={}", boundary),
        );

    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {}", token));
    }

    builder.body(Body::from(body)).expect("build request")
}

fn build_profile(auth_enabled: bool, token_env: &str) -> ServiceProfile {
    let profile_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("configs/profiles/api.jsonc");
    let mut profile = ServiceProfile::from_path(&profile_path).expect("load api profile");
    profile.auth.enabled = auth_enabled;
    profile.auth.dev_token_env = token_env.to_string();
    profile
}

#[tokio::test]
async fn auth_disabled_upload_works_without_token() {
    let profile = build_profile(false, "DOC_PARSER_AUTH_DISABLED_TEST");
    let app = build_app(build_state(profile));

    let resp = app
        .oneshot(multipart_upload_request(None))
        .await
        .expect("upload request");
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn auth_enabled_without_token_returns_401() {
    let profile = build_profile(true, "HOME");
    let app = build_app(build_state(profile));

    let resp = app
        .oneshot(multipart_upload_request(None))
        .await
        .expect("upload request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn auth_enabled_wrong_token_returns_401() {
    let profile = build_profile(true, "HOME");
    let app = build_app(build_state(profile));

    let resp = app
        .oneshot(multipart_upload_request(Some("wrong-token")))
        .await
        .expect("upload request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn auth_enabled_correct_token_upload_accepted() {
    let profile = build_profile(true, "HOME");
    let token = std::env::var("HOME").expect("HOME must be present in test env");
    let app = build_app(build_state(profile));

    let resp = app
        .oneshot(multipart_upload_request(Some(&token)))
        .await
        .expect("upload request");
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn auth_enabled_missing_env_returns_503() {
    let profile = build_profile(true, "DOC_PARSER_MISSING_AUTH_TOKEN_ENV");
    let app = build_app(build_state(profile));

    let resp = app
        .oneshot(multipart_upload_request(Some("any-token")))
        .await
        .expect("upload request");
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .expect("read response body");
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("AUTH_TOKEN_NOT_CONFIGURED"));
}
