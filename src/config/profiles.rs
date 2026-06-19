use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::load_jsonc_file;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub locale: String,
    pub default_language: String,
    pub max_concurrent_jobs: usize,
    pub job_queue_capacity: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub backend: String,
    pub input_dir: String,
    pub output_dir: String,
    pub metadata_backend: String,
    pub object_store_backend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub max_file_size_mb: u64,
    pub max_pages_per_document: usize,
    pub max_extracted_assets_mb: u64,
    pub max_image_width_px: u32,
    pub max_image_height_px: u32,
    pub max_archive_entries: usize,
    pub max_archive_total_uncompressed_mb: u64,
    pub max_processing_time_sec: u64,
    pub allow_external_converters: bool,
    pub allow_network_for_converters: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub tracing_enabled: bool,
    pub metrics_enabled: bool,
    pub prometheus_enabled: bool,
    pub prometheus_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthProfile {
    pub enabled: bool,
    pub dev_token_env: String,
}

impl Default for AuthProfile {
    fn default() -> Self {
        Self {
            enabled: false,
            dev_token_env: "DOC_PARSER_DEV_TOKEN".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceProfile {
    pub service: ServiceConfig,
    pub storage: StorageConfig,
    pub security: SecurityConfig,
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub auth: AuthProfile,
}

impl ServiceProfile {
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        load_jsonc_file(path)
    }

    pub fn from_default_profile(name: &str) -> anyhow::Result<Self> {
        let path = PathBuf::from("configs/profiles").join(format!("{}.jsonc", name));
        Self::from_path(&path)
    }
}
