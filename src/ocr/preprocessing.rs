use std::path::Path;

use anyhow::Context;
use image::{DynamicImage, ImageBuffer, Rgb, RgbImage, imageops::FilterType};

#[derive(Debug, Clone)]
pub struct PreprocessedImage {
    pub width: u32,
    pub height: u32,
    pub original_width: u32,
    pub original_height: u32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub pad_x: f32,
    pub pad_y: f32,
    pub data_f32_chw: Vec<f32>,
}

pub fn load_image_rgb(path: &Path) -> anyhow::Result<DynamicImage> {
    let image = image::open(path)
        .with_context(|| format!("failed to load image for OCR: {}", path.display()))?;
    Ok(DynamicImage::ImageRgb8(image.to_rgb8()))
}

pub fn resize_with_padding(
    image: &DynamicImage,
    target_width: u32,
    target_height: u32,
    padding_color: Rgb<u8>,
) -> anyhow::Result<PreprocessedImage> {
    let rgb = image.to_rgb8();
    let (src_w, src_h) = rgb.dimensions();

    if src_w == 0 || src_h == 0 || target_width == 0 || target_height == 0 {
        anyhow::bail!("invalid image dimensions for preprocessing");
    }

    let scale = (target_width as f32 / src_w as f32).min(target_height as f32 / src_h as f32);
    let resized_w = ((src_w as f32 * scale).round() as u32).max(1).min(target_width);
    let resized_h = ((src_h as f32 * scale).round() as u32).max(1).min(target_height);

    let resized = image::imageops::resize(&rgb, resized_w, resized_h, FilterType::CatmullRom);
    let mut canvas: RgbImage = ImageBuffer::from_pixel(target_width, target_height, padding_color);

    let pad_x = ((target_width - resized_w) / 2) as i64;
    let pad_y = ((target_height - resized_h) / 2) as i64;
    image::imageops::replace(&mut canvas, &resized, pad_x, pad_y);

    let mut data_f32_chw = vec![0.0_f32; (3 * target_width * target_height) as usize];
    for y in 0..target_height {
        for x in 0..target_width {
            let pixel = canvas.get_pixel(x, y).0;
            let idx = (y * target_width + x) as usize;
            data_f32_chw[idx] = pixel[0] as f32 / 255.0;
            data_f32_chw[(target_width * target_height) as usize + idx] = pixel[1] as f32 / 255.0;
            data_f32_chw[(2 * target_width * target_height) as usize + idx] = pixel[2] as f32 / 255.0;
        }
    }

    Ok(PreprocessedImage {
        width: target_width,
        height: target_height,
        original_width: src_w,
        original_height: src_h,
        scale_x: resized_w as f32 / src_w as f32,
        scale_y: resized_h as f32 / src_h as f32,
        pad_x: pad_x as f32,
        pad_y: pad_y as f32,
        data_f32_chw,
    })
}
