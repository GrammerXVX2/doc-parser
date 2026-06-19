use std::path::Path;
use std::time::Duration;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use document_parser::api::{build_app, build_state};
use document_parser::config::profiles::ServiceProfile;
use serde_json::Value;
use tower::ServiceExt;

fn upload_request() -> Request<Body> {
    let boundary = "x-boundary";
    let input_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/ru/sample_ru.html");
    let input = std::fs::read(input_path).expect("read sample input");

    let mut body = Vec::new();
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"sample_ru.html\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: text/html\r\n\r\n");
    body.extend_from_slice(&input);
    body.extend_from_slice(format!("\r\n--{}\r\n", boundary).as_bytes());

    body.extend_from_slice(b"Content-Disposition: form-data; name=\"language\"\r\n\r\nru");
    body.extend_from_slice(format!("\r\n--{}\r\n", boundary).as_bytes());

    body.extend_from_slice(b"Content-Disposition: form-data; name=\"extract_tables\"\r\n\r\ntrue");
    body.extend_from_slice(format!("\r\n--{}\r\n", boundary).as_bytes());

    body.extend_from_slice(b"Content-Disposition: form-data; name=\"table_chunks\"\r\n\r\ntrue");
    body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

    Request::builder()
        .method("POST")
        .uri("/v1/documents")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(body))
        .expect("build upload request")
}

#[tokio::test]
async fn upload_then_get_model_by_returned_document_id() {
    let profile_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("configs/profiles/api.jsonc");
    let mut profile = ServiceProfile::from_path(&profile_path).expect("load profile");
    profile.storage.input_dir = "target/api_e2e_input".to_string();
    profile.storage.output_dir = "target/api_e2e_output".to_string();
    profile.auth.enabled = false;

    std::fs::create_dir_all(&profile.storage.input_dir).expect("create input dir");
    std::fs::create_dir_all(&profile.storage.output_dir).expect("create output dir");

    let app = build_app(build_state(profile));

    let upload_resp = app
        .clone()
        .oneshot(upload_request())
        .await
        .expect("upload response");
    assert_eq!(upload_resp.status(), StatusCode::OK);

    let upload_body = to_bytes(upload_resp.into_body(), 1024 * 1024)
        .await
        .expect("read upload body");
    let upload_json: Value = serde_json::from_slice(&upload_body).expect("parse upload response");

    let job_id = upload_json
        .get("job_id")
        .and_then(Value::as_str)
        .expect("job_id in upload response")
        .to_string();
    let document_id = upload_json
        .get("document_id")
        .and_then(Value::as_str)
        .expect("document_id in upload response")
        .to_string();

    let mut final_status = String::new();
    for _ in 0..120 {
        let req = Request::builder()
            .method("GET")
            .uri(format!("/v1/jobs/{}", job_id))
            .body(Body::empty())
            .expect("build job request");
        let resp = app.clone().oneshot(req).await.expect("job status response");
        assert_eq!(resp.status(), StatusCode::OK);

        let body = to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .expect("read job body");
        let payload: Value = serde_json::from_slice(&body).expect("parse job payload");

        final_status = payload
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if matches!(final_status.as_str(), "completed" | "partial" | "failed") {
            break;
        }

        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    assert!(
        matches!(final_status.as_str(), "completed" | "partial"),
        "unexpected terminal job status: {}",
        final_status
    );

    let model_req = Request::builder()
        .method("GET")
        .uri(format!("/v1/documents/{}/model", document_id))
        .body(Body::empty())
        .expect("build model request");
    let model_resp = app
        .clone()
        .oneshot(model_req)
        .await
        .expect("model response");
    assert_eq!(model_resp.status(), StatusCode::OK);
}
