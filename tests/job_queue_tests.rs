use std::sync::Arc;
use std::time::Duration;

use document_parser::jobs::{InMemoryJobQueue, Job, JobRegistry, JobStatus, JobWorker, ProcessingOptions};
use document_parser::observability::MetricsRegistry;
use document_parser::storage::InMemoryMetadataStore;

#[tokio::test]
async fn enqueue_job_and_status_progression() {
    let metadata = Arc::new(InMemoryMetadataStore::new());
    let registry = JobRegistry::new(metadata);
    let metrics = Arc::new(MetricsRegistry::new());
    let worker = JobWorker::new(
        registry.clone(),
        std::path::PathBuf::from("target/job_queue_tests_output"),
        30,
        5000,
        2048,
        metrics,
    );
    let queue = InMemoryJobQueue::new(10, 1, worker);

    let input_path = std::path::PathBuf::from("testdata/ru/sample_ru.txt");
    let job = Job::new(
        "job_test_1".to_string(),
        "doc_test_1".to_string(),
        input_path,
        ProcessingOptions::default(),
    );
    registry.create(job.clone()).await.unwrap();
    queue.enqueue(job).await.unwrap();

    tokio::time::sleep(Duration::from_millis(300)).await;
    let maybe = registry.get("job_test_1").await.unwrap();
    assert!(maybe.is_some());
    let status = maybe.unwrap().status;
    assert!(
        matches!(status, JobStatus::Queued | JobStatus::Processing | JobStatus::Completed | JobStatus::Partial)
    );
}

#[tokio::test]
async fn queue_capacity_exposed() {
    let metadata = Arc::new(InMemoryMetadataStore::new());
    let registry = JobRegistry::new(metadata);
    let metrics = Arc::new(MetricsRegistry::new());
    let worker = JobWorker::new(
        registry,
        std::path::PathBuf::from("target/job_queue_tests_output"),
        30,
        5000,
        2048,
        metrics,
    );
    let queue = InMemoryJobQueue::new(2, 1, worker);
    assert_eq!(queue.capacity(), 2);
    assert!(queue.remaining_capacity() <= 2);
}
