use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::sync::Semaphore;
use tracing::{error, info, instrument};

use crate::api::errors::ApiErrorPayload;
use crate::config::{load_format_routing_config, load_pipeline_config};
use crate::jobs::job::Job;
use crate::jobs::progress::JobProgress;
use crate::jobs::registry::JobRegistry;
use crate::jobs::status::JobStatus;
use crate::model::ProcessingStatus;
use crate::observability::MetricsRegistry;
use crate::pipeline::{PipelineContext, run_pipeline};
use crate::writer::write_document_outputs;

#[derive(Clone)]
pub struct JobWorker {
    pub registry: JobRegistry,
    pub output_dir: PathBuf,
    pub processing_timeout_sec: u64,
    pub max_pages_per_document: usize,
    pub max_extracted_assets_mb: u64,
    pub metrics: Arc<MetricsRegistry>,
}

impl JobWorker {
    pub fn new(
        registry: JobRegistry,
        output_dir: PathBuf,
        processing_timeout_sec: u64,
        max_pages_per_document: usize,
        max_extracted_assets_mb: u64,
        metrics: Arc<MetricsRegistry>,
    ) -> Self {
        Self {
            registry,
            output_dir,
            processing_timeout_sec,
            max_pages_per_document,
            max_extracted_assets_mb,
            metrics,
        }
    }

    #[instrument(skip_all, fields(job_id = %job.job_id, document_id = %job.document_id))]
    pub async fn process_job(&self, mut job: Job) {
        let queue_wait_ms = (Utc::now() - job.created_at)
            .num_milliseconds()
            .max(0) as f64;
        self.metrics.observe_ms("job_queue_wait_ms", queue_wait_ms).await;

        job.status = JobStatus::Processing;
        job.updated_at = Utc::now();
        job.progress = JobProgress {
            stage: Some("processing".to_string()),
            pages_total: None,
            pages_processed: None,
            percent: Some(5.0),
        };
        if let Err(err) = self.registry.update(job.clone()).await {
            error!("failed to mark job processing: {}", err);
            return;
        }

        self.metrics.inc("jobs_processing").await;
        let started = std::time::Instant::now();

        let mut timed_out = false;
        let result = tokio::time::timeout(Duration::from_secs(self.processing_timeout_sec), async {
            crate::runtime::set_output_root_dir(self.output_dir.clone());
            let pipeline_config = load_pipeline_config(std::path::Path::new("configs/pipeline.config.jsonc"))?;
            let routing_config = load_format_routing_config(std::path::Path::new("configs/format_routing.config.jsonc"))?;
            let context = PipelineContext::new(pipeline_config, routing_config);
            let (_classification, mut model) = run_pipeline(&job.input_path, &context)?;

            if model.pages.len() > self.max_pages_per_document {
                return Err(anyhow::anyhow!("MAX_PAGES_LIMIT_EXCEEDED"));
            }

            model.document_id = job.document_id.clone();
            model.job_id = Some(job.job_id.clone());
            write_document_outputs(&model, &self.output_dir, true)?;

            let assets_size = sum_directory_size_bytes(
                &self
                    .output_dir
                    .join(&job.document_id)
                    .join("assets"),
            )?;
                model.document_id = job.document_id.clone();
            let max_assets = self.max_extracted_assets_mb.saturating_mul(1024 * 1024);
            if assets_size > max_assets {
                return Err(anyhow::anyhow!("OUTPUT_ASSETS_LIMIT_EXCEEDED"));
            }

            Ok::<ProcessingStatus, anyhow::Error>(model.processing.status)
        })
        .await;

        let final_status = match result {
            Ok(Ok(status)) => match status {
                ProcessingStatus::Ok => JobStatus::Completed,
                ProcessingStatus::Partial => JobStatus::Partial,
                ProcessingStatus::Failed => JobStatus::Failed,
            },
            Ok(Err(err)) => {
                let text = err.to_string();
                job.error = Some(if text == "MAX_PAGES_LIMIT_EXCEEDED" {
                    ApiErrorPayload {
                        code: "DOCUMENT_TOO_LARGE".to_string(),
                        message: "Количество страниц документа превышает допустимый лимит.".to_string(),
                        recoverable: false,
                        details: std::collections::HashMap::new(),
                    }
                } else if text == "OUTPUT_ASSETS_LIMIT_EXCEEDED" {
                    ApiErrorPayload {
                        code: "OUTPUT_ASSETS_LIMIT_EXCEEDED".to_string(),
                        message: "Общий размер извлеченных assets превышает допустимый лимит.".to_string(),
                        recoverable: false,
                        details: std::collections::HashMap::new(),
                    }
                } else {
                    ApiErrorPayload {
                        code: "PROCESSING_FAILED".to_string(),
                        message: format!("Ошибка обработки документа: {}", err),
                        recoverable: true,
                        details: std::collections::HashMap::new(),
                    }
                });
                JobStatus::Failed
            }
            Err(_) => {
                timed_out = true;
                job.error = Some(ApiErrorPayload {
                    code: "PROCESSING_TIMEOUT".to_string(),
                    message: "Превышено максимальное время обработки документа.".to_string(),
                    recoverable: false,
                    details: std::collections::HashMap::new(),
                });
                JobStatus::Failed
            }
        };

        let duration_ms = started.elapsed().as_millis() as f64;
        self.metrics
            .observe_ms("document_processing_duration_ms", duration_ms)
            .await;
        self.metrics.observe_ms("job_processing_ms", duration_ms).await;
        match final_status {
            JobStatus::Completed => self.metrics.inc("documents_completed_total").await,
            JobStatus::Partial => self.metrics.inc("documents_partial_total").await,
            JobStatus::Failed => self.metrics.inc("documents_failed_total").await,
            _ => {}
        }

        if timed_out {
            self.metrics.inc("errors_total_by_code_PROCESSING_TIMEOUT").await;
        }

        job.status = final_status;
        job.updated_at = Utc::now();
        job.progress = JobProgress {
            stage: Some("finished".to_string()),
            pages_total: None,
            pages_processed: None,
            percent: Some(100.0),
        };

        if let Err(err) = self.registry.update(job.clone()).await {
            error!("failed to update final job status: {}", err);
        }

        info!("job processing completed");
    }

    pub async fn spawn_bounded(
        &self,
        job: Job,
        semaphore: Arc<Semaphore>,
    ) {
        let worker = self.clone();
        tokio::spawn(async move {
            let permit = semaphore.acquire_owned().await;
            if permit.is_err() {
                return;
            }
            worker.process_job(job).await;
        });
    }
}

fn sum_directory_size_bytes(path: &std::path::Path) -> anyhow::Result<u64> {
    if !path.exists() {
        return Ok(0);
    }

    let mut total = 0_u64;
    let mut stack = vec![path.to_path_buf()];

    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let p = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(p);
            } else {
                total = total.saturating_add(entry.metadata()?.len());
            }
        }
    }

    Ok(total)
}
