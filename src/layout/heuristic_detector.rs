use async_trait::async_trait;

use crate::converters::traits::{ConversionError, ConversionStageRecord, ExtractionContext, Result};
use crate::layout::traits::{LayoutDetectionInput, LayoutDetector};
use crate::layout::types::{LayoutRegion, LayoutRegionType, LayoutSource};
use crate::model::ElementType;
use crate::utils::geometry::BBox;

#[derive(Debug, Default, Clone)]
pub struct HeuristicLayoutDetector;

#[async_trait]
impl LayoutDetector for HeuristicLayoutDetector {
    async fn detect_layout(
        &self,
        input: LayoutDetectionInput,
        context: &mut ExtractionContext,
    ) -> Result<Vec<LayoutRegion>> {
        let mut regions = Vec::new();

        for (idx, el) in input.existing_elements.iter().enumerate() {
            let Some(bbox_arr) = el.bbox else {
                continue;
            };
            let bbox = BBox::from_array(bbox_arr);
            let mut region_type = match el.element_type {
                ElementType::Heading => LayoutRegionType::Title,
                ElementType::List | ElementType::ListItem => LayoutRegionType::List,
                ElementType::Table => LayoutRegionType::Table,
                ElementType::Image | ElementType::PageImage | ElementType::Chart => LayoutRegionType::Figure,
                ElementType::Formula => LayoutRegionType::Formula,
                ElementType::Code => LayoutRegionType::Code,
                _ => LayoutRegionType::Text,
            };

            if bbox.y0 <= input.page_height * 0.08 && matches!(region_type, LayoutRegionType::Text) {
                region_type = LayoutRegionType::Header;
            }
            if bbox.y1 >= input.page_height * 0.92 && matches!(region_type, LayoutRegionType::Text) {
                region_type = LayoutRegionType::Footer;
            }
            if let Some(role) = el.role.as_deref() {
                if role.contains("watermark") {
                    region_type = LayoutRegionType::Watermark;
                }
            }

            regions.push(LayoutRegion {
                region_id: format!("r{}", idx + 1),
                page_number: input.page_number,
                region_type,
                bbox,
                polygon: None,
                confidence: 0.82,
                reading_order: el.reading_order.map(|v| v as usize),
                source: LayoutSource::Heuristic,
            });
        }

        if regions.is_empty() {
            regions.push(LayoutRegion {
                region_id: "r1".to_string(),
                page_number: input.page_number,
                region_type: LayoutRegionType::Unknown,
                bbox: BBox {
                    x0: 0.0,
                    y0: 0.0,
                    x1: input.page_width.max(1.0),
                    y1: input.page_height.max(1.0),
                },
                polygon: None,
                confidence: 0.4,
                reading_order: Some(1),
                source: LayoutSource::Heuristic,
            });
        }

        context.push_stage(
            ConversionStageRecord::ok("layout_detection", "heuristic_layout_detector")
                .with_meta("regions", regions.len().to_string())
                .with_meta("backend", "heuristic"),
        );

        if regions.is_empty() {
            return Err(ConversionError::new(
                "LAYOUT_DETECTION_FAILED",
                "Heuristic detector не вернул ни одного региона.",
            ));
        }

        Ok(regions)
    }
}
