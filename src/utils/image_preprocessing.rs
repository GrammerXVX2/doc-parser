use std::path::Path;

use image::{DynamicImage, Rgb};

use crate::ocr::preprocessing::{PreprocessedImage, load_image_rgb, resize_with_padding};

pub fn load_image_rgb_for_ocr(path: &Path) -> anyhow::Result<DynamicImage> {
    load_image_rgb(path)
}

pub fn resize_with_padding_for_ocr(
    image: &DynamicImage,
    target_width: u32,
    target_height: u32,
    padding_color: Rgb<u8>,
) -> anyhow::Result<PreprocessedImage> {
    resize_with_padding(image, target_width, target_height, padding_color)
}
