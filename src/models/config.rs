use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStackConfig {
    pub model_stack: ModelStackRoot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStackRoot {
    pub schema_version: String,
    pub default_language: String,
    pub fallback_languages: Vec<String>,
    pub locale: String,
    pub mode: String,
    pub global: ModelGlobalConfig,
    pub routing: ModelRoutingConfig,
    pub profiles: HashMap<String, ModelProfileConfig>,
    pub backends: HashMap<String, ModelBackendConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelGlobalConfig {
    pub enable_real_models: bool,
    pub fallback_to_mock: bool,
    pub fallback_to_fixture: bool,
    pub fail_on_missing_required_model: bool,
    pub save_model_debug_artifacts: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRoutingConfig {
    pub default_profile: String,
    pub auto_detect_domain: bool,
    pub allow_slow_path: bool,
    pub slow_path_confidence_threshold: f32,
    pub legal_required_fields_check: bool,
    pub prefer_native_extraction: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelProfileConfig {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub structured_document_parse: Value,
    #[serde(default)]
    pub ocr: Value,
    #[serde(default)]
    pub layout: Value,
    #[serde(default)]
    pub tables: Value,
    #[serde(default)]
    pub formulas: Value,
    #[serde(default)]
    pub legal: Value,
    #[serde(default)]
    pub book: Value,
    #[serde(default)]
    pub slow_path: Value,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelBackendConfig {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub backend_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub detection_model: Option<String>,
    #[serde(default)]
    pub recognition_model: Option<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub fallback: Vec<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub health_path: Option<String>,
    #[serde(default)]
    pub ocr_path: Option<String>,
    #[serde(default)]
    pub layout_path: Option<String>,
    #[serde(default)]
    pub table_path: Option<String>,
    #[serde(default)]
    pub parse_path: Option<String>,
    #[serde(default)]
    pub timeout_sec: Option<u64>,
    #[serde(default)]
    pub fallback_to_mock: Option<bool>,
    #[serde(default)]
    pub fallback_to_fixture: Option<bool>,
    #[serde(default)]
    pub fallback_to_heuristic: Option<bool>,
    #[serde(default)]
    pub fallback_to_placeholder: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl ModelBackendConfig {
    pub fn backend_url(&self) -> Option<String> {
        self.url
            .clone()
            .or_else(|| self.extra.get("url").and_then(|v| v.as_str()).map(ToOwned::to_owned))
    }

    pub fn endpoint_path(&self, key: &str, default: &str) -> String {
        let direct = match key {
            "health_path" => self.health_path.clone(),
            "ocr_path" => self.ocr_path.clone(),
            "layout_path" => self.layout_path.clone(),
            "table_path" => self.table_path.clone(),
            "parse_path" => self.parse_path.clone(),
            _ => None,
        };

        direct
            .or_else(|| self.extra.get(key).and_then(|v| v.as_str()).map(ToOwned::to_owned))
            .unwrap_or_else(|| default.to_string())
    }

    pub fn timeout(&self, default_sec: u64) -> Duration {
        let timeout = self
            .timeout_sec
            .or_else(|| self.extra.get("timeout_sec").and_then(|v| v.as_u64()))
            .unwrap_or(default_sec);
        Duration::from_secs(timeout)
    }

    pub fn fallback_to_mock_enabled(&self, default: bool) -> bool {
        self.fallback_to_mock
            .or_else(|| self.extra.get("fallback_to_mock").and_then(|v| v.as_bool()))
            .unwrap_or(default)
    }

    pub fn fallback_to_heuristic_enabled(&self, default: bool) -> bool {
        self.fallback_to_heuristic
            .or_else(|| self.extra.get("fallback_to_heuristic").and_then(|v| v.as_bool()))
            .unwrap_or(default)
    }

    pub fn fallback_to_placeholder_enabled(&self, default: bool) -> bool {
        self.fallback_to_placeholder
            .or_else(|| self.extra.get("fallback_to_placeholder").and_then(|v| v.as_bool()))
            .unwrap_or(default)
    }
}

impl Default for ModelStackConfig {
    fn default() -> Self {
        let mut profiles = HashMap::new();
        profiles.insert(
            "mixed_enterprise".to_string(),
            ModelProfileConfig {
                description: "Безопасный профиль по умолчанию (mock/fallback).".to_string(),
                ocr: serde_json::json!({
                    "primary": "paddleocr_ppocrv6_medium",
                    "fallback": ["surya_ocr", "mock_ocr"]
                }),
                layout: serde_json::json!({
                    "primary": "surya_layout",
                    "fallback": ["heuristic_layout"]
                }),
                tables: serde_json::json!({
                    "native": true,
                    "scanned": "table_transformer",
                    "fallback": ["placeholder_table"]
                }),
                formulas: serde_json::json!({
                    "native": true,
                    "scanned": "pix2tex",
                    "fallback": ["placeholder_formula"]
                }),
                slow_path: serde_json::json!({
                    "enabled": true,
                    "backend": "paddleocr_vl_1_6",
                    "alternatives": ["qwen3_vl", "granite_docling_258m"]
                }),
                ..ModelProfileConfig::default()
            },
        );

        let mut backends = HashMap::new();
        backends.insert(
            "paddleocr_ppocrv6_medium".to_string(),
            ModelBackendConfig {
                kind: "ocr".to_string(),
                enabled: true,
                backend_type: "external_command_or_http".to_string(),
                required: false,
                detection_model: Some("PaddlePaddle/PP-OCRv6_medium_det".to_string()),
                recognition_model: Some("PaddlePaddle/PP-OCRv6_medium_rec".to_string()),
                languages: vec!["ru".to_string(), "en".to_string()],
                ..ModelBackendConfig::default()
            },
        );
        backends.insert(
            "surya_ocr".to_string(),
            ModelBackendConfig {
                kind: "ocr".to_string(),
                enabled: true,
                backend_type: "external_command_or_http".to_string(),
                required: false,
                languages: vec!["ru".to_string(), "en".to_string()],
                ..ModelBackendConfig::default()
            },
        );

        Self {
            model_stack: ModelStackRoot {
                schema_version: "1.0.0".to_string(),
                default_language: "ru".to_string(),
                fallback_languages: vec!["ru".to_string(), "en".to_string()],
                locale: "ru".to_string(),
                mode: "dev".to_string(),
                global: ModelGlobalConfig {
                    enable_real_models: false,
                    fallback_to_mock: true,
                    fallback_to_fixture: true,
                    fail_on_missing_required_model: false,
                    save_model_debug_artifacts: false,
                },
                routing: ModelRoutingConfig {
                    default_profile: "mixed_enterprise".to_string(),
                    auto_detect_domain: true,
                    allow_slow_path: true,
                    slow_path_confidence_threshold: 0.65,
                    legal_required_fields_check: true,
                    prefer_native_extraction: true,
                },
                profiles,
                backends,
            },
        }
    }
}

pub fn load_model_stack_config(path: &Path) -> anyhow::Result<ModelStackConfig> {
    let raw = std::fs::read_to_string(path)
        .map_err(|err| anyhow::anyhow!("failed to read model stack config '{}': {err}", path.display()))?;
    let parsed: ModelStackConfig = json5::from_str(&raw)
        .map_err(|err| anyhow::anyhow!("failed to parse model stack config '{}': {err}", path.display()))?;
    Ok(parsed)
}

pub fn load_model_stack_config_or_default(path: Option<&Path>) -> ModelStackConfig {
    let config_path = path
        .map(|v| v.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("configs/model_stack.config.jsonc"));

    match load_model_stack_config(&config_path) {
        Ok(cfg) => cfg,
        Err(err) => {
            if !config_path.exists() {
                tracing::warn!(
                    code = "MODEL_STACK_CONFIG_MISSING",
                    path = %config_path.display(),
                    "Конфиг model stack не найден, используется safe default"
                );
            } else {
                tracing::warn!(
                    code = "MODEL_STACK_CONFIG_INVALID",
                    path = %config_path.display(),
                    error = %err,
                    "Конфиг model stack невалиден, используется safe default"
                );
            }
            ModelStackConfig::default()
        }
    }
}
