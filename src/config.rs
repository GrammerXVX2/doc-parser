use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::Context;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod profiles;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub pipeline: PipelineSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSection {
    pub version: String,
    pub mode: String,
    #[serde(default)]
    pub limits: Value,
    #[serde(default)]
    pub queues: Value,
    #[serde(default)]
    pub timeouts_ms: Value,
    #[serde(default)]
    pub rendering: Value,
    #[serde(default)]
    pub performance: Value,
    #[serde(default)]
    pub language: Value,
    #[serde(default)]
    pub locale: Value,
    #[serde(default)]
    pub pdf: Value,
    #[serde(default)]
    pub ocr: Value,
    #[serde(default)]
    pub ml: Value,
    #[serde(default)]
    pub tables: Value,
    #[serde(default)]
    pub formulas: Value,
    #[serde(default)]
    pub layout: Value,
    #[serde(default)]
    pub scanned_tables: Value,
    #[serde(default)]
    pub debug: Value,
    #[serde(default)]
    pub images: Value,
    #[serde(default)]
    pub merge: Value,
    #[serde(default)]
    pub chunking: Value,
    #[serde(default)]
    pub office: Value,
    #[serde(default)]
    pub presentation: Value,
    #[serde(default)]
    pub legacy: Value,
    #[serde(default)]
    pub converters: Value,
    #[serde(default)]
    pub office_rendering: Value,
    #[serde(default)]
    pub output: Value,
}

pub fn pipeline_pdf_value<'a>(config: &'a PipelineConfig, key: &str) -> Option<&'a Value> {
    config.pipeline.pdf.get(key)
}

pub fn pipeline_ocr_value<'a>(config: &'a PipelineConfig, key: &str) -> Option<&'a Value> {
    config.pipeline.ocr.get(key)
}

pub fn pipeline_performance_value<'a>(config: &'a PipelineConfig, key: &str) -> Option<&'a Value> {
    config.pipeline.performance.get(key)
}

pub fn pipeline_ml_value<'a>(config: &'a PipelineConfig, key: &str) -> Option<&'a Value> {
    config.pipeline.ml.get(key)
}

pub fn pipeline_merge_value<'a>(config: &'a PipelineConfig, key: &str) -> Option<&'a Value> {
    config.pipeline.merge.get(key)
}

pub fn pipeline_language_value<'a>(config: &'a PipelineConfig, key: &str) -> Option<&'a Value> {
    config.pipeline.language.get(key)
}

pub fn pipeline_locale_value<'a>(config: &'a PipelineConfig, key: &str) -> Option<&'a Value> {
    config.pipeline.locale.get(key)
}

pub fn pipeline_office_value<'a>(config: &'a PipelineConfig, key: &str) -> Option<&'a Value> {
    config.pipeline.office.get(key)
}

pub fn pipeline_presentation_value<'a>(config: &'a PipelineConfig, key: &str) -> Option<&'a Value> {
    config.pipeline.presentation.get(key)
}

pub fn pipeline_legacy_value<'a>(config: &'a PipelineConfig, key: &str) -> Option<&'a Value> {
    config.pipeline.legacy.get(key)
}

pub fn pipeline_converters_value<'a>(config: &'a PipelineConfig, key: &str) -> Option<&'a Value> {
    config.pipeline.converters.get(key)
}

pub fn pipeline_layout_value<'a>(config: &'a PipelineConfig, key: &str) -> Option<&'a Value> {
    config.pipeline.layout.get(key)
}

pub fn pipeline_scanned_tables_value<'a>(config: &'a PipelineConfig, key: &str) -> Option<&'a Value> {
    config.pipeline.scanned_tables.get(key)
}

pub fn pipeline_debug_value<'a>(config: &'a PipelineConfig, key: &str) -> Option<&'a Value> {
    config.pipeline.debug.get(key)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatRoutingConfig {
    pub routing: HashMap<String, RoutingRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub mime_types: Vec<String>,
    #[serde(default)]
    pub primary_stages: Vec<String>,
    #[serde(default)]
    pub fallback_stages: Vec<String>,
    #[serde(default)]
    pub tools: HashMap<String, Vec<String>>,
}

pub fn load_jsonc_file<T>(path: &Path) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;
    let parsed: T = json5::from_str(&raw)
        .with_context(|| format!("failed to parse JSONC/JSON5: {}", path.display()))?;
    Ok(parsed)
}

pub fn load_pipeline_config(path: &Path) -> anyhow::Result<PipelineConfig> {
    load_jsonc_file(path)
}

pub fn load_format_routing_config(path: &Path) -> anyhow::Result<FormatRoutingConfig> {
    load_jsonc_file(path)
}
