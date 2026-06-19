use std::sync::Arc;

use crate::jobs::job::Job;
use crate::storage::MetadataStore;

#[derive(Clone)]
pub struct JobRegistry {
    metadata: Arc<dyn MetadataStore>,
}

impl JobRegistry {
    pub fn new(metadata: Arc<dyn MetadataStore>) -> Self {
        Self { metadata }
    }

    pub async fn create(&self, job: Job) -> anyhow::Result<()> {
        self.metadata.create_job(job).await
    }

    pub async fn update(&self, job: Job) -> anyhow::Result<()> {
        self.metadata.update_job(job).await
    }

    pub async fn get(&self, job_id: &str) -> anyhow::Result<Option<Job>> {
        self.metadata.get_job(job_id).await
    }

    pub async fn by_document_id(&self, document_id: &str) -> anyhow::Result<Option<Job>> {
        self.metadata.find_document_job(document_id).await
    }
}
