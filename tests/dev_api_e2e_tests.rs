use std::path::PathBuf;
use std::sync::Arc;

use axum::body::to_bytes;
use document_parser::api::{build_app, build_state};
use document_parser::config::profiles::ServiceProfile;
use document_parser::jobs::{Job, JobRegistry, JobWorker, ProcessingOptions};
use document_parser::observability::MetricsRegistry;
use document_parser::storage::InMemoryMetadataStore;
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
async fn upload_returns_document_id_and_model_is_retrievable() {
    let mut profile = ServiceProfile::from_path(&PathBuf::from("configs/profiles/dev_team.jsonc")).unwrap();
    profile.storage.output_dir = format!(
        "target/dev_api_e2e_output_{}",
        uuid::Uuid::new_v4().simple()
    );

    let app = build_app(build_state(profile.clone()));
    let boundary = "x-boundary";

    let upload_req = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/documents")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .body(axum::body::Body::from(multipart_body(
            boundary,
            "sample_ru.html",
            &std::fs::read("testdata/ru/sample_ru.html").unwrap(),
        )))
        .unwrap();

    let upload_resp = app.clone().oneshot(upload_req).await.unwrap();
    assert_eq!(upload_resp.status(), axum::http::StatusCode::OK);
    let upload_body = to_bytes(upload_resp.into_body(), usize::MAX).await.unwrap();
    let upload_json: serde_json::Value = serde_json::from_slice(&upload_body).unwrap();

    let document_id = upload_json
        .get("document_id")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string();
    let job_id = upload_json
        .get("job_id")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string();

    let input_path = PathBuf::from(&profile.storage.input_dir)
        .join(&document_id)
        .join("sample_ru.html");
    let registry = JobRegistry::new(Arc::new(InMemoryMetadataStore::new()));
    let worker = JobWorker::new(
        registry.clone(),
        PathBuf::from(&profile.storage.output_dir),
        120,
        profile.security.max_pages_per_document,
        profile.security.max_extracted_assets_mb,
        Arc::new(MetricsRegistry::new()),
    );

    let job = Job::new(job_id, document_id.clone(), input_path, ProcessingOptions::default());
    registry.create(job.clone()).await.unwrap();
    worker.process_job(job).await;

    let model_resp = app
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/v1/documents/{}/model", document_id))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(model_resp.status(), axum::http::StatusCode::OK);
}