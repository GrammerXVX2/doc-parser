use async_trait::async_trait;

use crate::converters::traits::{ConversionStageRecord, ExtractionContext, Result};
use crate::layout::traits::{LayoutDetectionInput, LayoutDetector};
use crate::layout::types::{LayoutRegion, LayoutRegionType, LayoutSource};
use crate::model::ElementType;
use crate::utils::geometry::BBox;

#[derive(Debug, Default, Clone)]
pub struct MockLayoutDetector;

#[async_trait]
impl LayoutDetector for MockLayoutDetector {
    async fn detect_layout(
        &self,
        input: LayoutDetectionInput,
        context: &mut ExtractionContext,
    ) -> Result<Vec<LayoutRegion>> {
        let mut regions = Vec::new();

        if input.existing_elements.iter().any(|e| matches!(e.element_type, ElementType::Table)) {
            regions.push(LayoutRegion {
                region_id: "mock_table_1".to_string(),
                page_number: input.page_number,
                region_type: LayoutRegionType::Table,
                bbox: BBox {
                    x0: input.page_width * 0.1,
                    y0: input.page_height * 0.3,
                    x1: input.page_width * 0.9,
                    y1: input.page_height * 0.7,
                },
                polygon: None,
                confidence: 0.88,
                reading_order: Some(2),
                source: LayoutSource::Mock,
            });
        }

        if input
            .existing_elements
            .iter()
            .any(|e| matches!(e.element_type, ElementType::Formula))
        {
            regions.push(LayoutRegion {
                region_id: "mock_formula_1".to_string(),
                page_number: input.page_number,
                region_type: LayoutRegionType::Formula,
                bbox: BBox {
                    x0: input.page_width * 0.2,
                    y0: input.page_height * 0.72,
                    x1: input.page_width * 0.8,
                    y1: input.page_height * 0.82,
                },
                polygon: None,
                confidence: 0.84,
                reading_order: Some(3),
                source: LayoutSource::Mock,
            });
        }

        if regions.is_empty() {
            regions.push(LayoutRegion {
                region_id: "mock_text_1".to_string(),
                page_number: input.page_number,
                region_type: LayoutRegionType::Text,
                bbox: BBox {
                    x0: input.page_width * 0.08,
                    y0: input.page_height * 0.08,
                    x1: input.page_width * 0.92,
                    y1: input.page_height * 0.35,
                },
                polygon: None,
                confidence: 0.8,
                reading_order: Some(1),
                source: LayoutSource::Mock,
            });
        }

        context.push_stage(
            ConversionStageRecord::ok("layout_detection", "mock_layout_detector")
                .with_meta("regions", regions.len().to_string())
                .with_meta("backend", "mock"),
        );

        Ok(regions)
    }
}
