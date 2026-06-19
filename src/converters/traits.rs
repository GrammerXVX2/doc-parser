use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct ConversionError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
    pub metadata: HashMap<String, String>,
}

impl ConversionError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            recoverable: true,
            metadata: HashMap::new(),
        }
    }

    pub fn with_recoverable(mut self, recoverable: bool) -> Self {
        self.recoverable = recoverable;
        self
    }

    pub fn with_meta(mut self, key: &str, value: impl Into<String>) -> Self {
        self.metadata.insert(key.to_string(), value.into());
        self
    }
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ConversionError {}

pub type Result<T> = std::result::Result<T, ConversionError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionTarget {
    Docx,
    Pdf,
    Html,
    Markdown,
    Text,
}

impl ConversionTarget {
    pub fn extension(self) -> &'static str {
        match self {
            ConversionTarget::Docx => "docx",
            ConversionTarget::Pdf => "pdf",
            ConversionTarget::Html => "html",
            ConversionTarget::Markdown => "md",
            ConversionTarget::Text => "txt",
        }
    }

    pub fn mime_type(self) -> &'static str {
        match self {
            ConversionTarget::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            ConversionTarget::Pdf => "application/pdf",
            ConversionTarget::Html => "text/html",
            ConversionTarget::Markdown => "text/markdown",
            ConversionTarget::Text => "text/plain",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ConversionTarget::Docx => "docx",
            ConversionTarget::Pdf => "pdf",
            ConversionTarget::Html => "html",
            ConversionTarget::Markdown => "markdown",
            ConversionTarget::Text => "text",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConvertedDocument {
    pub path: PathBuf,
    pub target: ConversionTarget,
    pub mime_type: String,
    pub converter_name: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct ExtractionContext {
    pub locale: String,
    pub stage_records: Vec<ConversionStageRecord>,
    pub warnings: Vec<ConversionError>,
    pub errors: Vec<ConversionError>,
}

impl Default for ExtractionContext {
    fn default() -> Self {
        Self {
            locale: "ru".to_string(),
            stage_records: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }
}

impl ExtractionContext {
    pub fn push_stage(&mut self, record: ConversionStageRecord) {
        self.stage_records.push(record);
    }

    pub fn push_warning(&mut self, warning: ConversionError) {
        self.warnings.push(warning);
    }

    pub fn push_error(&mut self, error: ConversionError) {
        self.errors.push(error);
    }
}

#[derive(Debug, Clone)]
pub struct ConversionStageRecord {
    pub name: String,
    pub status: String,
    pub tool: String,
    pub metadata: HashMap<String, String>,
}

impl ConversionStageRecord {
    pub fn ok(name: &str, tool: &str) -> Self {
        Self {
            name: name.to_string(),
            status: "ok".to_string(),
            tool: tool.to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn warning(name: &str, tool: &str) -> Self {
        Self {
            name: name.to_string(),
            status: "warning".to_string(),
            tool: tool.to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn error(name: &str, tool: &str) -> Self {
        Self {
            name: name.to_string(),
            status: "error".to_string(),
            tool: tool.to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_meta(mut self, key: &str, value: impl Into<String>) -> Self {
        self.metadata.insert(key.to_string(), value.into());
        self
    }
}

#[async_trait]
pub trait DocumentConverter: Send + Sync {
    fn name(&self) -> &'static str;

    fn supports_conversion(&self, input_path: &Path, target: ConversionTarget) -> bool;

    async fn convert(
        &self,
        input_path: &Path,
        target: ConversionTarget,
        context: &mut ExtractionContext,
    ) -> Result<ConvertedDocument>;
}
