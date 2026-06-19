use std::fs;
use std::path::Path;

use anyhow::Context;

pub fn read_image_bytes(path: &Path) -> anyhow::Result<Vec<u8>> {
    fs::read(path).with_context(|| format!("failed to read image bytes: {}", path.display()))
}

pub fn image_dimensions(bytes: &[u8]) -> anyhow::Result<(u32, u32)> {
    let image = image::load_from_memory(bytes).with_context(|| "failed to decode image")?;
    Ok((image.width(), image.height()))
}
