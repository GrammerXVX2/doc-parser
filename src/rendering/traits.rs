use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RenderImageFormat {
    Png,
    Jpeg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderOptions {
    pub dpi: u32,
    pub max_width_px: Option<u32>,
    pub max_height_px: Option<u32>,
    pub format: RenderImageFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedPage {
    pub page_number: usize,
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    pub asset_id: String,
    pub path: String,
    pub mime_type: String,
    #[serde(skip)]
    pub bytes: Vec<u8>,
}

pub trait PageRenderer {
    fn render_page(
        &self,
        input_path: &Path,
        page_number: usize,
        options: RenderOptions,
    ) -> anyhow::Result<RenderedPage>;
}
