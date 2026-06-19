use std::fs;

use async_trait::async_trait;
use serde::Deserialize;

use crate::converters::traits::{ConversionError, ConversionStageRecord, ExtractionContext, Result};
use crate::layout::traits::{LayoutDetectionInput, LayoutDetector};
use crate::layout::types::{LayoutRegion, LayoutRegionType, LayoutSource};
use crate::utils::geometry::BBox;

#[derive(Debug, Default, Clone)]
pub struct FixtureLayoutDetector;

#[derive(Debug, Deserialize)]
struct FixtureRegion {
    #[serde(rename = "type")]
    region_type: String,
    bbox: [f32; 4],
    #[serde(default = "default_confidence")]
    confidence: f32,
}

fn default_confidence() -> f32 {
    0.9
}

#[async_trait]
impl LayoutDetector for FixtureLayoutDetector {
    async fn detect_layout(
        &self,
        input: LayoutDetectionInput,
        context: &mut ExtractionContext,
    ) -> Result<Vec<LayoutRegion>> {
        let Some(base_path) = input.page_image_path else {
            return Err(ConversionError::new(
                "LAYOUT_FIXTURE_NOT_FOUND",
                "Для fixture backend не передан page_image_path.",
            ));
        };

        let fixture_path = if let Some(ext) = base_path.extension().and_then(|e| e.to_str()) {
            base_path.with_extension(format!("{}.layout.json", ext))
        } else {
            base_path.with_extension("layout.json")
        };

        if !fixture_path.exists() {
            return Err(ConversionError::new(
                "LAYOUT_FIXTURE_NOT_FOUND",
                format!(
                    "Файл fixture layout не найден: {}",
                    fixture_path.to_string_lossy()
                ),
            ));
        }

        let raw = fs::read_to_string(&fixture_path).map_err(|err| {
            ConversionError::new(
                "LAYOUT_DETECTION_FAILED",
                format!("Не удалось прочитать fixture layout: {}", err),
            )
        })?;

        let parsed: Vec<FixtureRegion> = serde_json::from_str(&raw).map_err(|err| {
            ConversionError::new(
                "LAYOUT_DETECTION_FAILED",
                format!("Некорректный формат fixture layout JSON: {}", err),
            )
        })?;

        let regions = parsed
            .into_iter()
            .enumerate()
            .map(|(idx, item)| LayoutRegion {
                region_id: format!("fixture_{}", idx + 1),
                page_number: input.page_number,
                region_type: LayoutRegionType::from_str(&item.region_type),
                bbox: BBox::from_array(item.bbox),
                polygon: None,
                confidence: item.confidence,
                reading_order: None,
                source: LayoutSource::Fixture,
            })
            .collect::<Vec<_>>();

        context.push_stage(
            ConversionStageRecord::ok("layout_detection", "fixture_layout_detector")
                .with_meta("regions", regions.len().to_string())
                .with_meta("backend", "fixture")
                .with_meta("fixture_path", fixture_path.to_string_lossy()),
        );

        Ok(regions)
    }
}
