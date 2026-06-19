use async_trait::async_trait;

use crate::jobs::job::Job;

pub type StoreResult<T> = Result<T, anyhow::Error>;

#[async_trait]
pub trait MetadataStore: Send + Sync {
    async fn create_job(&self, job: Job) -> StoreResult<()>;
    async fn update_job(&self, job: Job) -> StoreResult<()>;
    async fn get_job(&self, job_id: &str) -> StoreResult<Option<Job>>;
    async fn find_document_job(&self, document_id: &str) -> StoreResult<Option<Job>>;
}
