use std::path::PathBuf;
use std::sync::OnceLock;

static OUTPUT_ROOT_DIR: OnceLock<PathBuf> = OnceLock::new();
static OCR_CLI_OVERRIDES: OnceLock<OcrCliOverrides> = OnceLock::new();
static PIPELINE_CLI_OVERRIDES: OnceLock<PipelineCliOverrides> = OnceLock::new();

#[derive(Debug, Clone, Default)]
pub struct OcrCliOverrides {
    pub backend: Option<String>,
    pub det_model: Option<String>,
    pub rec_model: Option<String>,
    pub charset: Option<String>,
    pub provider: Option<String>,
    pub triton_url: Option<String>,
    pub save_crops: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct PipelineCliOverrides {
    pub language: Option<String>,
    pub languages: Option<Vec<String>>,
    pub locale: Option<String>,
    pub normalize_ru: Option<String>,
    pub extract_tables: Option<bool>,
    pub table_chunks: Option<bool>,
    pub layout_backend: Option<String>,
    pub detect_scanned_tables: Option<bool>,
    pub detect_formulas: Option<bool>,
    pub debug_layout: Option<bool>,
    pub reading_order: Option<String>,
    pub exclude_headers_footers_from_chunks: Option<bool>,
    pub model_stack_config: Option<String>,
    pub model_profile: Option<String>,
    pub domain: Option<String>,
    pub enable_slow_path: Option<bool>,
    pub execute_slow_path: Option<bool>,
    pub legal_extract: Option<bool>,
    pub book_extract: Option<bool>,
}

pub fn set_output_root_dir(path: PathBuf) {
    let _ = OUTPUT_ROOT_DIR.set(path);
}

pub fn output_root_dir() -> PathBuf {
    OUTPUT_ROOT_DIR
        .get()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("output"))
}

pub fn set_ocr_cli_overrides(overrides: OcrCliOverrides) {
    let _ = OCR_CLI_OVERRIDES.set(overrides);
}

pub fn ocr_cli_overrides() -> Option<&'static OcrCliOverrides> {
    OCR_CLI_OVERRIDES.get()
}

pub fn set_pipeline_cli_overrides(overrides: PipelineCliOverrides) {
    let _ = PIPELINE_CLI_OVERRIDES.set(overrides);
}

pub fn pipeline_cli_overrides() -> Option<&'static PipelineCliOverrides> {
    PIPELINE_CLI_OVERRIDES.get()
}
