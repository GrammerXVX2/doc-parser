use image::{DynamicImage, Rgb, RgbImage};

use document_parser::ocr::crop::CropExtractor;
use document_parser::ocr::types::TextRegion;
use document_parser::utils::geometry::BBox;

#[test]
fn valid_bbox_crop_works() {
    let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(100, 100, Rgb([0, 0, 0])));
    let regions = vec![TextRegion {
        bbox: BBox {
            x0: 10.0,
            y0: 20.0,
            x1: 40.0,
            y1: 60.0,
        },
        polygon: None,
        confidence: 0.9,
        orientation_degrees: 0.0,
    }];

    let crops = CropExtractor::crop_regions(&image, &regions).expect("crop should succeed");
    assert_eq!(crops.len(), 1);
    assert_eq!(crops[0].image.width(), 30);
    assert_eq!(crops[0].image.height(), 40);
}

#[test]
fn bbox_outside_image_is_clamped() {
    let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(100, 100, Rgb([0, 0, 0])));
    let regions = vec![TextRegion {
        bbox: BBox {
            x0: -20.0,
            y0: -10.0,
            x1: 120.0,
            y1: 110.0,
        },
        polygon: None,
        confidence: 0.9,
        orientation_degrees: 0.0,
    }];

    let crops = CropExtractor::crop_regions(&image, &regions).expect("crop should succeed");
    assert_eq!(crops.len(), 1);
    assert_eq!(crops[0].image.width(), 100);
    assert_eq!(crops[0].image.height(), 100);
}

#[test]
fn empty_bbox_is_skipped() {
    let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(100, 100, Rgb([0, 0, 0])));
    let regions = vec![TextRegion {
        bbox: BBox {
            x0: 50.0,
            y0: 50.0,
            x1: 50.0,
            y1: 50.0,
        },
        polygon: None,
        confidence: 0.9,
        orientation_degrees: 0.0,
    }];

    let crops = CropExtractor::crop_regions(&image, &regions).expect("crop should succeed");
    assert!(crops.is_empty());
}
