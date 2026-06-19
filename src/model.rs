use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::language::LanguageInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentModel {
    pub schema_version: String,
    pub document_id: String,
    pub job_id: Option<String>,
    pub source: SourceInfo,
    pub document_profile: DocumentProfile,
    pub stats: DocumentStats,
    pub coordinate_system: CoordinateSystem,
    #[serde(default)]
    pub assets: Vec<Asset>,
    #[serde(default)]
    pub pages: Vec<Page>,
    #[serde(default)]
    pub chunks: Vec<Chunk>,
    #[serde(default)]
    pub errors: Vec<Diagnostic>,
    #[serde(default)]
    pub warnings: Vec<Diagnostic>,
    pub processing: ProcessingTrace,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub uri: String,
    pub filename: String,
    pub extension: String,
    pub mime_type: String,
    pub size_bytes: Option<u64>,
    pub hashes: Hashes,
    pub uploaded_at: Option<DateTime<Utc>>,
    pub processed_at: Option<DateTime<Utc>>,
    pub container: SourceContainer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hashes {
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceContainer {
    #[serde(rename = "type")]
    pub container_type: Option<String>,
    pub parent_uri: Option<String>,
    pub entry_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentProfile {
    pub format: DocumentFormat,
    pub content_mode: ContentMode,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub language_info: LanguageInfo,
    pub has_native_text: bool,
    pub has_images: bool,
    pub has_tables: bool,
    pub has_formulas: bool,
    pub has_ocr_required_regions: bool,
    pub has_handwriting: bool,
    pub has_multicolumn_layout: bool,
    pub document_type_guess: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentFormat {
    Pdf,
    Docx,
    Doc,
    Html,
    Md,
    Rtf,
    Image,
    Pptx,
    Txt,
    Xlsx,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentMode {
    Digital,
    Scanned,
    Hybrid,
    Image,
    Spreadsheet,
    Presentation,
    PlainText,
    Sheet,
    Slide,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentStats {
    pub page_count: u32,
    pub element_count: u32,
    pub text_element_count: u32,
    pub ocr_element_count: u32,
    pub image_count: u32,
    pub table_count: u32,
    pub formula_count: u32,
    pub list_count: u32,
    pub total_chars: u32,
    pub total_words: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinateSystem {
    pub origin: String,
    pub unit: String,
    pub dpi: Option<u32>,
    pub normalized_to_page: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub asset_id: String,
    #[serde(rename = "type")]
    pub asset_type: String,
    pub path: String,
    pub mime_type: String,
    pub page_number: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub dpi: Option<u32>,
    pub sha256: Option<String>,
    pub provenance: Value,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub page_number: u32,
    pub page_type: PageType,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub dpi: Option<u32>,
    pub rotation_degrees: f32,
    pub page_image_asset_id: Option<String>,
    pub page_profile: PageProfile,
    #[serde(default)]
    pub elements: Vec<Element>,
    pub text: String,
    pub markdown: String,
    pub html: String,
    #[serde(default)]
    pub warnings: Vec<Diagnostic>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PageType {
    DocumentPage,
    Slide,
    Sheet,
    Image,
    SyntheticTextPage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageProfile {
    pub content_mode: ContentMode,
    pub has_native_text: bool,
    pub has_ocr_required_regions: bool,
    pub has_tables: bool,
    pub has_images: bool,
    pub has_formulas: bool,
    pub has_handwriting: bool,
    pub language: Option<String>,
    #[serde(default)]
    pub language_info: LanguageInfo,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    pub element_id: String,
    #[serde(rename = "type")]
    pub element_type: ElementType,
    pub tag: Option<String>,
    pub role: Option<String>,
    pub reading_order: Option<u32>,
    pub global_order: Option<u32>,
    pub bbox: Option<[f32; 4]>,
    pub polygon: Option<Vec<[f32; 2]>>,
    #[serde(default)]
    pub content: Value,
    #[serde(default)]
    pub style: Value,
    #[serde(default)]
    pub provenance: Value,
    #[serde(default)]
    pub confidence: Value,
    #[serde(default)]
    pub warnings: Vec<Diagnostic>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ElementType {
    Text,
    TextOcr,
    Heading,
    Paragraph,
    Blockquote,
    List,
    ListItem,
    Code,
    Image,
    PageImage,
    Table,
    Formula,
    Caption,
    Header,
    Footer,
    Footnote,
    Watermark,
    Chart,
    Shape,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub chunk_id: String,
    #[serde(rename = "type")]
    pub chunk_type: String,
    pub title: Option<String>,
    #[serde(default)]
    pub section_path: Vec<String>,
    pub page_start: u32,
    pub page_end: u32,
    #[serde(default)]
    pub element_ids: Vec<String>,
    pub text: String,
    pub markdown: String,
    pub token_estimate: u32,
    pub metadata: Value,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: String,
    pub scope: String,
    pub page_number: Option<u32>,
    pub element_id: Option<String>,
    pub message: String,
    #[serde(default)]
    pub recoverable: bool,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingTrace {
    pub pipeline_version: String,
    pub status: ProcessingStatus,
    #[serde(default)]
    pub stages: Vec<ProcessingStage>,
    pub total_duration_ms: Option<u64>,
    pub runtime: RuntimeInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingStatus {
    Ok,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStage {
    pub name: String,
    pub status: StageStatus,
    pub tool: String,
    pub duration_ms: Option<u64>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    Ok,
    Skipped,
    Warning,
    Error,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeInfo {
    pub hostname: Option<String>,
    pub cpu: Option<String>,
    pub gpu: Option<String>,
    pub cuda_version: Option<String>,
    pub onnxruntime_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageLocale {
    Ru,
    En,
}
