use std::time::Instant;

use image::{DynamicImage, Rgb, RgbImage};

use crate::ocr::crop::OcrCrop;
use crate::ocr::traits::{TextDetector, TextRecognizer};
use crate::ocr::types::{OcrPageInput, TextRegion};
use crate::utils::geometry::BBox;

#[derive(Debug, Clone)]
pub struct WarmupReport {
    pub model_name: String,
    pub iterations: usize,
    pub duration_ms: u64,
    pub status: String,
}

pub fn warmup_detector(
    detector: &(dyn TextDetector + Send + Sync),
    iterations: usize,
) -> anyhow::Result<WarmupReport> {
    let iters = iterations.max(1);
    let started = Instant::now();

    let warmup_file = std::env::temp_dir().join(format!(
        "doc_parser_warmup_detector_{}.png",
        uuid::Uuid::new_v4()
    ));
    let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(64, 64, Rgb([255, 255, 255])));
    img.save(&warmup_file)?;

    let input = OcrPageInput {
        document_id: "warmup".to_string(),
        page_number: 1,
        image_asset_id: "warmup_asset".to_string(),
        image_path: warmup_file.clone(),
        width: 64,
        height: 64,
        dpi: Some(72),
    };

    for _ in 0..iters {
        let _ = detector.detect_page(&input)?;
    }

    let _ = std::fs::remove_file(warmup_file);

    Ok(WarmupReport {
        model_name: "ocr_detector".to_string(),
        iterations: iters,
        duration_ms: started.elapsed().as_millis() as u64,
        status: "ok".to_string(),
    })
}

pub fn warmup_recognizer(
    recognizer: &(dyn TextRecognizer + Send + Sync),
    iterations: usize,
) -> anyhow::Result<WarmupReport> {
    let iters = iterations.max(1);
    let started = Instant::now();

    let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(96, 32, Rgb([255, 255, 255])));
    let crop = OcrCrop {
        region: TextRegion {
            bbox: BBox {
                x0: 0.0,
                y0: 0.0,
                x1: 96.0,
                y1: 32.0,
            },
            polygon: None,
            confidence: 1.0,
            orientation_degrees: 0.0,
        },
        image,
        crop_index: 0,
    };

    for _ in 0..iters {
        let _ = recognizer.recognize_batch(vec![crop.clone()])?;
    }

    Ok(WarmupReport {
        model_name: "ocr_recognizer".to_string(),
        iterations: iters,
        duration_ms: started.elapsed().as_millis() as u64,
        status: "ok".to_string(),
    })
}
