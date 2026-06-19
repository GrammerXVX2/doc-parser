use std::path::Path;

use async_trait::async_trait;

use crate::config::PipelineConfig;
use crate::converters::{
    ConversionError, ConversionTarget, ConvertedDocument, DocumentConverter, ExtractionContext,
    LibreOfficeConverter,
};

#[async_trait]
pub trait OfficeRenderer: Send + Sync {
    async fn render_to_pdf(
        &self,
        input_path: &Path,
        context: &mut ExtractionContext,
    ) -> Result<ConvertedDocument, ConversionError>;
}

#[derive(Debug, Clone)]
pub struct LibreOfficeOfficeRenderer {
    converter: LibreOfficeConverter,
}

impl LibreOfficeOfficeRenderer {
    pub fn from_pipeline_config(config: Option<&PipelineConfig>) -> Self {
        Self {
            converter: LibreOfficeConverter::from_pipeline_config(config),
        }
    }
}

#[async_trait]
impl OfficeRenderer for LibreOfficeOfficeRenderer {
    async fn render_to_pdf(
        &self,
        input_path: &Path,
        context: &mut ExtractionContext,
    ) -> Result<ConvertedDocument, ConversionError> {
        self.converter
            .convert(input_path, ConversionTarget::Pdf, context)
            .await
    }
}
