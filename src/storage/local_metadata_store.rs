use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::jobs::job::Job;
use crate::storage::metadata_store::{MetadataStore, StoreResult};

#[derive(Debug, Default)]
pub struct InMemoryMetadataStore {
    jobs: Arc<RwLock<HashMap<String, Job>>>,
}

impl InMemoryMetadataStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl MetadataStore for InMemoryMetadataStore {
    async fn create_job(&self, job: Job) -> StoreResult<()> {
        self.jobs.write().await.insert(job.job_id.clone(), job);
        Ok(())
    }

    async fn update_job(&self, job: Job) -> StoreResult<()> {
        self.jobs.write().await.insert(job.job_id.clone(), job);
        Ok(())
    }

    async fn get_job(&self, job_id: &str) -> StoreResult<Option<Job>> {
        Ok(self.jobs.read().await.get(job_id).cloned())
    }

    async fn find_document_job(&self, document_id: &str) -> StoreResult<Option<Job>> {
        Ok(self
            .jobs
            .read()
            .await
            .values()
            .find(|job| job.document_id == document_id)
            .cloned())
    }
}

#[derive(Debug)]
pub struct LocalJsonMetadataStore {
    pub root: PathBuf,
}

impl LocalJsonMetadataStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn file_for(&self, job_id: &str) -> PathBuf {
        self.root.join("jobs").join(format!("{}.json", job_id))
    }
}

#[async_trait]
impl MetadataStore for LocalJsonMetadataStore {
    async fn create_job(&self, job: Job) -> StoreResult<()> {
        self.update_job(job).await
    }

    async fn update_job(&self, job: Job) -> StoreResult<()> {
        let file = self.file_for(&job.job_id);
        if let Some(parent) = file.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let bytes = serde_json::to_vec_pretty(&job)?;
        tokio::fs::write(file, bytes).await?;
        Ok(())
    }

    async fn get_job(&self, job_id: &str) -> StoreResult<Option<Job>> {
        let file = self.file_for(job_id);
        if !file.exists() {
            return Ok(None);
        }
        let bytes = tokio::fs::read(file).await?;
        let job = serde_json::from_slice::<Job>(&bytes)?;
        Ok(Some(job))
    }

    async fn find_document_job(&self, document_id: &str) -> StoreResult<Option<Job>> {
        let jobs_dir = self.root.join("jobs");
        if !jobs_dir.exists() {
            return Ok(None);
        }
        let mut dir = tokio::fs::read_dir(jobs_dir).await?;
        while let Some(entry) = dir.next_entry().await? {
            if entry.file_type().await?.is_file() {
                let bytes = tokio::fs::read(entry.path()).await?;
                let job = serde_json::from_slice::<Job>(&bytes)?;
                if job.document_id == document_id {
                    return Ok(Some(job));
                }
            }
        }
        Ok(None)
    }
}
