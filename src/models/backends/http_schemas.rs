use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrHttpRequest {
    pub document_id: String,
    pub page_number: u32,
    pub image_path: String,
    pub languages: Vec<String>,
    #[serde(default)]
    pub options: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrHttpResponse {
    pub backend: String,
    #[serde(default)]
    pub regions: Vec<OcrHttpRegion>,
    pub text: Option<String>,
    pub confidence: Option<f32>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrHttpRegion {
    pub text: String,
    pub bbox: [f32; 4],
    pub confidence: f32,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutHttpRequest {
    pub document_id: String,
    pub page_number: u32,
    pub image_path: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    #[serde(default)]
    pub options: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutHttpResponse {
    pub backend: String,
    #[serde(default)]
    pub regions: Vec<LayoutHttpRegion>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutHttpRegion {
    pub region_type: String,
    pub bbox: [f32; 4],
    pub confidence: f32,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredParseHttpRequest {
    pub document_id: String,
    pub input_path: String,
    pub format: String,
    pub language: String,
    #[serde(default)]
    pub options: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredParseHttpResponse {
    pub backend: String,
    pub markdown: Option<String>,
    pub text: Option<String>,
    #[serde(default)]
    pub elements: Value,
    #[serde(default)]
    pub tables: Value,
    #[serde(default)]
    pub metadata: Value,
    pub confidence: Option<f32>,
}
