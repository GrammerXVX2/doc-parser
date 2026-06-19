use std::path::PathBuf;

use document_parser::api::{build_app, build_state};
use document_parser::config::profiles::ServiceProfile;
use document_parser::security::{SecurityLimits, validate_upload};
use tower::ServiceExt;

#[test]
fn missing_input_file_returns_structured_error() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input = root.join("testdata/ru/does_not_exist.txt");

    let pipeline = document_parser::config::load_pipeline_config(&root.join("configs/pipeline.config.jsonc"))
        .unwrap();
    let routing = document_parser::config::load_format_routing_config(
        &root.join("configs/format_routing.config.jsonc"),
    )
    .unwrap();
    let context = document_parser::pipeline::PipelineContext::new(pipeline, routing);

    let err = document_parser::pipeline::run_pipeline(&input, &context)
        .expect_err("expected missing file error")
        .to_string();
    assert!(
        err.contains("failed to read input file")
            || err.contains("failed to stat input file")
            || err.contains("No such file")
    );
}

#[test]
fn empty_file_returns_russian_structured_error() {
    let limits = SecurityLimits::default();
    let err = validate_upload("sample.txt", b"", &limits)
        .expect_err("expected empty file error")
        .payload;
    assert_eq!(err.code, "EMPTY_FILE");
    assert!(err.message.contains("Файл пустой"));
}

#[test]
fn unsupported_extension_returns_russian_structured_error() {
    let limits = SecurityLimits::default();
    let err = validate_upload("sample.exe", b"abc", &limits)
        .expect_err("expected unsupported type")
        .payload;
    assert_eq!(err.code, "UNSUPPORTED_FILE_TYPE");
    assert!(err.message.contains("не поддерживается"));
}

#[test]
fn corrupted_ooxml_is_reported() {
    let limits = SecurityLimits::default();
    let err = validate_upload("sample.docx", b"not-a-zip", &limits)
        .expect_err("expected archive parse error")
        .payload;
    assert_eq!(err.code, "UNSUPPORTED_FILE_TYPE");
}

#[tokio::test]
async fn queue_full_returns_structured_error() {
    let mut profile = ServiceProfile::from_path(&PathBuf::from("configs/profiles/api.jsonc")).unwrap();
    profile.service.job_queue_capacity = 0;

    let app = build_app(build_state(profile));
    let boundary = "x-boundary";
    let body = {
        let mut body = Vec::new();
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            b"Content-Disposition: form-data; name=\"file\"; filename=\"sample.md\"\r\n",
        );
        body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
        body.extend_from_slice(b"# test");
        body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());
        body
    };

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/documents")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .body(axum::body::Body::from(body))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);
}

#[test]
fn triton_unavailable_is_structured() {
    let client = document_parser::ml::TritonClient::new("http://127.0.0.1:1").unwrap();
    let err = client.ensure_ready().expect_err("triton should be unavailable").to_string();
    assert!(err.contains("TRITON_UNAVAILABLE"));
}
