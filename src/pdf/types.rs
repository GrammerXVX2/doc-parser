use serde::{Deserialize, Serialize};

use crate::model::ContentMode;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PdfPageContentMode {
    Digital,
    Scanned,
    Hybrid,
    Empty,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfPageClassification {
    pub page_number: usize,
    pub content_mode: PdfPageContentMode,
    pub has_native_text: bool,
    pub native_text_chars: usize,
    pub has_images: bool,
    pub image_area_ratio: Option<f32>,
    pub requires_ocr: bool,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfClassification {
    pub page_count: usize,
    pub encrypted: bool,
    pub pages: Vec<PdfPageClassification>,
    pub document_content_mode: ContentMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfPageNativeText {
    pub page_number: usize,
    pub text: String,
    pub synthetic_bbox: Option<[f32; 4]>,
}
