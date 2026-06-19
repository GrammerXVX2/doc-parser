use image::{DynamicImage, GenericImageView};

use crate::ocr::types::TextRegion;
use crate::utils::geometry::BBox;

#[derive(Debug, Clone)]
pub struct OcrCrop {
    pub region: TextRegion,
    pub image: DynamicImage,
    pub crop_index: usize,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CropExtractor;

impl CropExtractor {
    pub fn crop_regions(image: &DynamicImage, regions: &[TextRegion]) -> anyhow::Result<Vec<OcrCrop>> {
        let (width, height) = image.dimensions();
        let mut out = Vec::new();

        for (idx, region) in regions.iter().enumerate() {
            let clamped = clamp_bbox(region.bbox, width as f32, height as f32);
            if clamped.x1 <= clamped.x0 || clamped.y1 <= clamped.y0 {
                continue;
            }

            let x = clamped.x0.floor() as u32;
            let y = clamped.y0.floor() as u32;
            let w = (clamped.x1.ceil() as u32).saturating_sub(x).max(1);
            let h = (clamped.y1.ceil() as u32).saturating_sub(y).max(1);

            let cropped = image.crop_imm(x, y, w.min(width.saturating_sub(x)), h.min(height.saturating_sub(y)));
            out.push(OcrCrop {
                region: TextRegion {
                    bbox: clamped,
                    polygon: region.polygon.clone(),
                    confidence: region.confidence,
                    orientation_degrees: region.orientation_degrees,
                },
                image: cropped,
                crop_index: idx,
            });
        }

        Ok(out)
    }
}

fn clamp_bbox(bbox: BBox, width: f32, height: f32) -> BBox {
    BBox {
        x0: bbox.x0.clamp(0.0, width),
        y0: bbox.y0.clamp(0.0, height),
        x1: bbox.x1.clamp(0.0, width),
        y1: bbox.y1.clamp(0.0, height),
    }
}
