use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::api::errors::ApiErrorPayload;
use crate::jobs::progress::JobProgress;
use crate::jobs::status::JobStatus;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessingOptions {
    pub language: Option<String>,
    pub languages: Option<Vec<String>>,
    pub ocr_backend: Option<String>,
    pub extract_tables: Option<bool>,
    pub table_chunks: Option<bool>,
    pub layout_backend: Option<String>,
    pub model_stack_config: Option<String>,
    pub model_profile: Option<String>,
    pub domain: Option<String>,
    pub enable_slow_path: Option<bool>,
    pub execute_slow_path: Option<bool>,
    pub legal_extract: Option<bool>,
    pub book_extract: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub job_id: String,
    pub document_id: String,
    pub input_path: PathBuf,
    pub status: JobStatus,
    pub progress: JobProgress,
    pub options: ProcessingOptions,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub error: Option<ApiErrorPayload>,
}

impl Job {
    pub fn new(job_id: String, document_id: String, input_path: PathBuf, options: ProcessingOptions) -> Self {
        let now = Utc::now();
        Self {
            job_id,
            document_id,
            input_path,
            status: JobStatus::Queued,
            progress: JobProgress::default(),
            options,
            created_at: now,
            updated_at: now,
            error: None,
        }
    }
}
