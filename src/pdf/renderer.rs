use std::path::Path;

use anyhow::Context;

use crate::assets::{AssetStore, AssetType, LocalAssetStore};
use crate::config::PipelineConfig;
use crate::rendering::traits::{PageRenderer, RenderImageFormat, RenderOptions, RenderedPage};
use crate::runtime::output_root_dir;

pub fn render_pdf_page_if_needed(
    renderer: &dyn PageRenderer,
    input_path: &Path,
    page_number: usize,
    document_id: &str,
    pipeline_config: Option<&PipelineConfig>,
) -> anyhow::Result<RenderedPage> {
    let dpi = pipeline_config
        .and_then(|cfg| cfg.pipeline.pdf.get("default_render_dpi"))
        .and_then(|v| v.as_u64())
        .unwrap_or(144) as u32;

    let options = RenderOptions {
        dpi,
        max_width_px: None,
        max_height_px: None,
        format: RenderImageFormat::Png,
    };

    let mut rendered = renderer
        .render_page(input_path, page_number, options)
        .with_context(|| format!("failed to render pdf page {}", page_number))?;

    let asset_store = LocalAssetStore::new(output_root_dir());
    let asset = asset_store.write_asset(
        document_id,
        AssetType::PageRender,
        &format!("page_{}.png", page_number),
        &rendered.bytes,
        &rendered.mime_type,
    )?;

    rendered.asset_id = asset.asset_id;
    rendered.path = asset.path;

    Ok(rendered)
}
