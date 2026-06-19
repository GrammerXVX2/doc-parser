use document_parser::jobs::{Job, ProcessingOptions};
use document_parser::storage::{InMemoryMetadataStore, LocalJsonMetadataStore, MetadataStore};

#[tokio::test]
async fn in_memory_create_update_get_find() {
    let store = InMemoryMetadataStore::new();
    let mut job = Job::new(
        "job_meta_1".to_string(),
        "doc_meta_1".to_string(),
        std::path::PathBuf::from("testdata/ru/sample_ru.txt"),
        ProcessingOptions::default(),
    );

    store.create_job(job.clone()).await.unwrap();
    let loaded = store.get_job("job_meta_1").await.unwrap();
    assert!(loaded.is_some());

    job.status = document_parser::jobs::JobStatus::Processing;
    store.update_job(job.clone()).await.unwrap();

    let by_doc = store.find_document_job("doc_meta_1").await.unwrap();
    assert!(by_doc.is_some());
    assert_eq!(by_doc.unwrap().job_id, "job_meta_1");
}

#[tokio::test]
async fn local_json_store_roundtrip() {
    let root = std::path::PathBuf::from("target/metadata_store_tests");
    let _ = std::fs::remove_dir_all(&root);
    let store = LocalJsonMetadataStore::new(&root);

    let job = Job::new(
        "job_meta_json_1".to_string(),
        "doc_meta_json_1".to_string(),
        std::path::PathBuf::from("testdata/ru/sample_ru.txt"),
        ProcessingOptions::default(),
    );

    store.create_job(job.clone()).await.unwrap();
    let loaded = store.get_job("job_meta_json_1").await.unwrap();
    assert!(loaded.is_some());

    let by_doc = store.find_document_job("doc_meta_json_1").await.unwrap();
    assert!(by_doc.is_some());
    assert_eq!(by_doc.unwrap().job_id, "job_meta_json_1");
}
