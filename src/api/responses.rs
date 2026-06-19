use serde::Serialize;

use crate::jobs::{JobProgress, JobStatus};

#[derive(Debug, Serialize)]
pub struct UploadAcceptedResponse {
    pub job_id: String,
    pub document_id: String,
    pub status: JobStatus,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct JobStatusResponse {
    pub job_id: String,
    pub document_id: String,
    pub status: JobStatus,
    pub progress: JobProgress,
    pub created_at: String,
    pub updated_at: String,
    pub error: Option<crate::api::errors::ApiErrorPayload>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ReadyResponse {
    pub status: String,
    pub queue_capacity_remaining: usize,
    pub workers: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SystemPerformanceResponse {
    pub provider: String,
    pub ocr_backend: String,
    pub batching_enabled: bool,
    pub warmup_completed: bool,
}
