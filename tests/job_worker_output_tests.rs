use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use document_parser::api::{build_app, build_state};
use document_parser::config::profiles::ServiceProfile;
use document_parser::jobs::{InMemoryJobQueue, Job, JobRegistry, JobStatus, JobWorker, ProcessingOptions};
use document_parser::observability::MetricsRegistry;
use document_parser::storage::InMemoryMetadataStore;
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn worker_writes_output_to_job_document_id_and_api_reads_model() {
    let metadata = Arc::new(InMemoryMetadataStore::new());
    let registry = JobRegistry::new(metadata);
    let metrics = Arc::new(MetricsRegistry::new());

    let output_dir = PathBuf::from("target/job_worker_output_tests");
    if output_dir.exists() {
        std::fs::remove_dir_all(&output_dir).expect("cleanup output dir");
    }
    std::fs::create_dir_all(&output_dir).expect("create output dir");

    let worker = JobWorker::new(
        registry.clone(),
        output_dir.clone(),
        60,
        5000,
        2048,
        metrics,
    );
    let queue = InMemoryJobQueue::new(10, 1, worker);

    let document_id = "doc_test".to_string();
    let job_id = "job_test_output".to_string();
    let input_path = PathBuf::from("testdata/ru/sample_ru.html");
    let job = Job::new(
        job_id.clone(),
        document_id.clone(),
        input_path,
        ProcessingOptions::default(),
    );

    registry.create(job.clone()).await.expect("create job");
    queue.enqueue(job).await.expect("enqueue job");

    let mut terminal_status = JobStatus::Queued;
    for _ in 0..120 {
        tokio::time::sleep(Duration::from_millis(250)).await;
        let current = registry
            .get(&job_id)
            .await
            .expect("get job")
            .expect("job exists");
        terminal_status = current.status.clone();
        if matches!(
            terminal_status,
            JobStatus::Completed | JobStatus::Partial | JobStatus::Failed
        ) {
            break;
        }
    }

    assert!(
        matches!(terminal_status, JobStatus::Completed | JobStatus::Partial),
        "unexpected terminal status: {:?}",
        terminal_status
    );

    let model_path = output_dir.join("doc_test").join("model.json");
    assert!(
        model_path.exists(),
        "model must be written under output/doc_test/model.json"
    );

    let model: Value = serde_json::from_slice(
        &std::fs::read(&model_path).expect("read model.json"),
    )
    .expect("parse model json");
    assert_eq!(
        model.get("document_id").and_then(Value::as_str),
        Some("doc_test")
    );

    let profile_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("configs/profiles/api.jsonc");
    let mut profile = ServiceProfile::from_path(&profile_path).expect("load api profile");
    profile.storage.output_dir = output_dir.to_string_lossy().to_string();
    profile.auth.enabled = false;

    let app = build_app(build_state(profile));
    let req = Request::builder()
        .method("GET")
        .uri("/v1/documents/doc_test/model")
        .body(Body::empty())
        .expect("request build");
    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::OK);
}
