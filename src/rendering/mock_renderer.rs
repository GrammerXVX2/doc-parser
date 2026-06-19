use std::path::Path;

use anyhow::Context;
use image::{ImageBuffer, Rgba};

use super::traits::{PageRenderer, RenderImageFormat, RenderOptions, RenderedPage};

#[derive(Debug, Default)]
pub struct MockRenderer;

impl PageRenderer for MockRenderer {
    fn render_page(
        &self,
        _input_path: &Path,
        page_number: usize,
        options: RenderOptions,
    ) -> anyhow::Result<RenderedPage> {
        let width = 1190_u32;
        let height = 1684_u32;

        let mut image = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width, height);
        for (_, _, pixel) in image.enumerate_pixels_mut() {
            *pixel = Rgba([255, 255, 255, 255]);
        }

        let mut png_bytes = Vec::new();
        let format = match options.format {
            RenderImageFormat::Png => image::ImageFormat::Png,
            RenderImageFormat::Jpeg => image::ImageFormat::Jpeg,
        };

        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut std::io::Cursor::new(&mut png_bytes), format)
            .with_context(|| "failed to encode mock rendered image")?;

        let mime = match options.format {
            RenderImageFormat::Png => "image/png",
            RenderImageFormat::Jpeg => "image/jpeg",
        }
        .to_string();

        Ok(RenderedPage {
            page_number,
            width,
            height,
            dpi: options.dpi,
            asset_id: String::new(),
            path: String::new(),
            mime_type: mime,
            bytes: png_bytes,
        })
    }
}
