use std::path::PathBuf;

use async_trait::async_trait;

use crate::converters::traits::{ExtractionContext, Result};
use crate::layout::types::LayoutRegion;
use crate::model::Element;

#[derive(Debug, Clone)]
pub struct LayoutDetectionInput {
    pub document_id: String,
    pub page_number: usize,
    pub page_image_asset_id: Option<String>,
    pub page_image_path: Option<PathBuf>,
    pub page_width: f32,
    pub page_height: f32,
    pub existing_elements: Vec<Element>,
}

#[async_trait]
pub trait LayoutDetector: Send + Sync {
    async fn detect_layout(
        &self,
        input: LayoutDetectionInput,
        context: &mut ExtractionContext,
    ) -> Result<Vec<LayoutRegion>>;
}
