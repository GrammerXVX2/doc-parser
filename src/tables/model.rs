use serde::{Deserialize, Serialize};

use crate::utils::geometry::BBox;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCell {
    pub row: usize,
    pub column: usize,
    pub rowspan: usize,
    pub colspan: usize,
    pub bbox: Option<BBox>,
    pub text: String,
    pub html: Option<String>,
    pub markdown: Option<String>,
    #[serde(default)]
    pub formula: Option<String>,
    pub is_header: bool,
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStructure {
    pub has_header: bool,
    pub has_merged_cells: bool,
    pub orientation: String,
    pub extraction_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableLinearizedChunk {
    pub title: String,
    pub text: String,
    pub markdown: String,
    pub row_start: usize,
    pub row_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableLinearizationOptions {
    pub max_rows_per_chunk: usize,
    pub language: String,
}

impl Default for TableLinearizationOptions {
    fn default() -> Self {
        Self {
            max_rows_per_chunk: 20,
            language: "ru".to_string(),
        }
    }
}
