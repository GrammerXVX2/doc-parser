use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use tokio::sync::RwLock;

use crate::config::{load_pipeline_config, pipeline_performance_value};
use crate::config::profiles::ServiceProfile;
use crate::jobs::{InMemoryJobQueue, JobRegistry, JobWorker};
use crate::ml::TritonClient;
use crate::ocr::{OcrBackendKind, OcrConfig};
use crate::observability::MetricsRegistry;
use crate::runtime::set_output_root_dir;
use crate::security::SecurityLimits;
use crate::storage::{InMemoryMetadataStore, LocalDocumentStore, LocalJsonMetadataStore, MetadataStore};

#[derive(Clone)]
pub struct AppState {
    pub profile: ServiceProfile,
    pub queue: InMemoryJobQueue,
    pub registry: JobRegistry,
    pub document_store: LocalDocumentStore,
    pub metrics: Arc<MetricsRegistry>,
    pub workers: usize,
    pub serving: Arc<RwLock<bool>>,
    pub security_limits: SecurityLimits,
    pub performance_provider: String,
    pub ocr_backend: String,
    pub batching_enabled: bool,
    pub warmup_completed: bool,
    pub config_valid: bool,
    pub storage_ready: bool,
    pub model_registry_ready: bool,
}

impl AppState {
    pub fn queue_remaining(&self) -> usize {
        self.queue.remaining_capacity()
    }

    pub fn is_ready(&self) -> bool {
        self.config_valid
            && self.storage_ready
            && self.model_registry_ready
            && !self.queue.is_closed()
            && self.queue_remaining() > 0
    }
}

pub fn build_state(profile: ServiceProfile) -> AppState {
    set_output_root_dir(PathBuf::from(profile.storage.output_dir.clone()));

    let metrics = Arc::new(MetricsRegistry::new());
    let metadata: Arc<dyn MetadataStore> = match profile.storage.metadata_backend.as_str() {
        "local_json" => Arc::new(LocalJsonMetadataStore::new("data/metadata")),
        _ => Arc::new(InMemoryMetadataStore::new()),
    };
    let registry = JobRegistry::new(metadata);

    let worker = JobWorker::new(
        registry.clone(),
        PathBuf::from(profile.storage.output_dir.clone()),
        profile.security.max_processing_time_sec,
        profile.security.max_pages_per_document,
        profile.security.max_extracted_assets_mb,
        metrics.clone(),
    );

    let queue = InMemoryJobQueue::new(
        profile.service.job_queue_capacity,
        profile.service.max_concurrent_jobs,
        worker,
    );

    let pipeline_config = load_pipeline_config(std::path::Path::new("configs/pipeline.config.jsonc")).ok();
    let config_valid = pipeline_config.is_some();
    let performance_provider = pipeline_config
        .as_ref()
        .and_then(|cfg| cfg.pipeline.ml.get("provider").and_then(|v| v.as_str()))
        .unwrap_or("cpu")
        .to_string();
    let ocr_backend = pipeline_config
        .as_ref()
        .and_then(|cfg| cfg.pipeline.ocr.get("backend").and_then(|v| v.as_str()))
        .unwrap_or("mock")
        .to_string();
    let batching_enabled = pipeline_config
        .as_ref()
        .and_then(|cfg| {
            pipeline_performance_value(cfg, "batching")
                .and_then(|v| v.get("enabled"))
                .and_then(|v| v.as_bool())
        })
        .unwrap_or(false);
    let warmup_completed = pipeline_config
        .as_ref()
        .and_then(|cfg| {
            pipeline_performance_value(cfg, "warmup")
                .and_then(|v| v.get("enabled"))
                .and_then(|v| v.as_bool())
        })
        .unwrap_or(false);

    let storage_ready = ensure_storage_ready(&profile);
    let model_registry_ready = evaluate_model_registry_ready(pipeline_config.as_ref());

    AppState {
        security_limits: SecurityLimits {
            max_file_size_mb: profile.security.max_file_size_mb,
            max_pages_per_document: profile.security.max_pages_per_document,
            max_extracted_assets_mb: profile.security.max_extracted_assets_mb,
            max_image_width_px: profile.security.max_image_width_px,
            max_image_height_px: profile.security.max_image_height_px,
            max_archive_entries: profile.security.max_archive_entries,
            max_archive_total_uncompressed_mb: profile.security.max_archive_total_uncompressed_mb,
            max_processing_time_sec: profile.security.max_processing_time_sec,
            allow_external_converters: profile.security.allow_external_converters,
            allow_network_for_converters: profile.security.allow_network_for_converters,
        },
        profile: profile.clone(),
        queue,
        registry,
        document_store: LocalDocumentStore::new(profile.storage.input_dir.clone()),
        metrics,
        workers: profile.service.max_concurrent_jobs,
        serving: Arc::new(RwLock::new(true)),
        performance_provider,
        ocr_backend,
        batching_enabled,
        warmup_completed,
        config_valid,
        storage_ready,
        model_registry_ready,
    }
}

fn ensure_storage_ready(profile: &ServiceProfile) -> bool {
    let input = PathBuf::from(&profile.storage.input_dir);
    let output = PathBuf::from(&profile.storage.output_dir);

    let input_ok = std::fs::create_dir_all(&input).is_ok();
    let output_ok = std::fs::create_dir_all(&output).is_ok();
    if !input_ok || !output_ok {
        return false;
    }

    let probe = output.join(format!(".ready_probe_{}", uuid::Uuid::new_v4()));
    match std::fs::write(&probe, b"ok") {
        Ok(_) => {
            let _ = std::fs::remove_file(probe);
            true
        }
        Err(_) => false,
    }
}

fn evaluate_model_registry_ready(pipeline: Option<&crate::config::PipelineConfig>) -> bool {
    let Some(pipeline) = pipeline else {
        return false;
    };

    let ocr = OcrConfig::from_pipeline_ocr_value(&pipeline.pipeline.ocr);
    if !ocr.enabled {
        return true;
    }

    if matches!(ocr.backend, OcrBackendKind::Onnx) {
        let models_exist = std::path::Path::new(&ocr.detection.model_path).exists()
            && std::path::Path::new(&ocr.recognition.model_path).exists()
            && std::path::Path::new(&ocr.recognition.charset_path).exists();
        if !models_exist && !ocr.fallback_to_mock {
            return false;
        }
    }

    let provider = pipeline
        .pipeline
        .ml
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("cpu");
    let triton_enabled = provider.eq_ignore_ascii_case("triton")
        || pipeline
            .pipeline
            .ml
            .get("triton")
            .and_then(|v| v.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

    if triton_enabled {
        let url = pipeline
            .pipeline
            .ml
            .get("triton")
            .and_then(|v| v.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("http://127.0.0.1:8000")
            .to_string();

        if TritonClient::new(url).and_then(|client| client.ensure_ready()).is_err() {
            return false;
        }
    }

    true
}

pub fn build_app(state: AppState) -> Router {
    crate::api::routes::build_router(state)
}

pub async fn run_server(profile: ServiceProfile) -> anyhow::Result<()> {
    let host = profile.service.host.clone();
    let port = profile.service.port;
    let state = build_state(profile);
    let app = build_app(state);
    let listener = tokio::net::TcpListener::bind((host.as_str(), port)).await?;
    tracing::info!("HTTP server listening on {}:{}", host, port);
    axum::serve(listener, app).await?;
    Ok(())
}
