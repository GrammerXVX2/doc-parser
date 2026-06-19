use serde::{Deserialize, Serialize};

use crate::utils::geometry::BBox;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfTextSpan {
    pub text: String,
    pub bbox: BBox,
    pub font_size: Option<f32>,
    pub font_name: Option<String>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfTextLine {
    pub spans: Vec<PdfTextSpan>,
    pub text: String,
    pub bbox: BBox,
    pub baseline_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfTextBlock {
    pub lines: Vec<PdfTextLine>,
    pub text: String,
    pub bbox: BBox,
    pub role_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PdfTextReconstructionOptions {
    pub line_y_tolerance: f32,
    pub word_gap_ratio: f32,
    pub paragraph_gap_ratio: f32,
    pub detect_headings_by_font: bool,
}

impl Default for PdfTextReconstructionOptions {
    fn default() -> Self {
        Self {
            line_y_tolerance: 0.012,
            word_gap_ratio: 0.6,
            paragraph_gap_ratio: 1.8,
            detect_headings_by_font: true,
        }
    }
}
