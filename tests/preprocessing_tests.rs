use image::{DynamicImage, Rgb, RgbImage};

use document_parser::ocr::preprocessing::resize_with_padding;

#[test]
fn resize_with_padding_preserves_aspect_and_output_shape() {
    let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(400, 200, Rgb([10, 20, 30])));
    let pre = resize_with_padding(&img, 320, 320, Rgb([255, 255, 255])).expect("preprocessing should work");

    assert_eq!(pre.width, 320);
    assert_eq!(pre.height, 320);
    assert_eq!(pre.original_width, 400);
    assert_eq!(pre.original_height, 200);
    assert!((pre.scale_x - 0.8).abs() < 0.01);
    assert!((pre.scale_y - 0.8).abs() < 0.01);
    assert!(pre.pad_y > 0.0);
    assert_eq!(pre.data_f32_chw.len(), 3 * 320 * 320);
    assert!(pre.data_f32_chw.iter().all(|v| *v >= 0.0 && *v <= 1.0));
}
