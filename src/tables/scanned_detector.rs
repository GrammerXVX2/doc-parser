use std::path::PathBuf;

use async_trait::async_trait;

use crate::converters::traits::{ConversionError, ConversionStageRecord, ExtractionContext, Result};
use crate::utils::geometry::BBox;

#[derive(Debug, Clone)]
pub struct TableDetectionInput {
    pub document_id: String,
    pub page_number: usize,
    pub page_image_path: Option<PathBuf>,
    pub page_width: f32,
    pub page_height: f32,
}

#[derive(Debug, Clone)]
pub struct TableRegion {
    pub region_id: String,
    pub page_number: usize,
    pub bbox: BBox,
    pub confidence: f32,
    pub source: String,
}

#[async_trait]
pub trait ScannedTableDetector: Send + Sync {
    async fn detect_tables(
        &self,
        input: TableDetectionInput,
        context: &mut ExtractionContext,
    ) -> Result<Vec<TableRegion>>;
}

#[derive(Debug, Default, Clone)]
pub struct MockScannedTableDetector;

#[async_trait]
impl ScannedTableDetector for MockScannedTableDetector {
    async fn detect_tables(
        &self,
        input: TableDetectionInput,
        context: &mut ExtractionContext,
    ) -> Result<Vec<TableRegion>> {
        let regions = vec![TableRegion {
            region_id: "table_mock_1".to_string(),
            page_number: input.page_number,
            bbox: BBox {
                x0: input.page_width * 0.12,
                y0: input.page_height * 0.28,
                x1: input.page_width * 0.88,
                y1: input.page_height * 0.72,
            },
            confidence: 0.82,
            source: "mock_scanned_table_detector".to_string(),
        }];
        context.push_stage(
            ConversionStageRecord::ok("scanned_table_detection", "mock_scanned_table_detector")
                .with_meta("regions", regions.len().to_string()),
        );
        Ok(regions)
    }
}

#[derive(Debug, Default, Clone)]
pub struct FixtureScannedTableDetector;

#[async_trait]
impl ScannedTableDetector for FixtureScannedTableDetector {
    async fn detect_tables(
        &self,
        input: TableDetectionInput,
        context: &mut ExtractionContext,
    ) -> Result<Vec<TableRegion>> {
        let Some(path) = input.page_image_path else {
            return Err(ConversionError::new(
                "SCANNED_TABLE_DETECTION_FAILED",
                "Для fixture detector отсутствует page_image_path.",
            ));
        };

        let fixture = if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            path.with_extension(format!("{}.layout.json", ext))
        } else {
            path.with_extension("layout.json")
        };

        if !fixture.exists() {
            return Err(ConversionError::new(
                "SCANNED_TABLE_DETECTION_FAILED",
                format!(
                    "Fixture table regions не найден: {}",
                    fixture.to_string_lossy()
                ),
            ));
        }

        let raw = std::fs::read_to_string(&fixture).map_err(|err| {
            ConversionError::new(
                "SCANNED_TABLE_DETECTION_FAILED",
                format!("Не удалось прочитать fixture file: {}", err),
            )
        })?;

        let parsed: Vec<serde_json::Value> = serde_json::from_str(&raw).map_err(|err| {
            ConversionError::new(
                "SCANNED_TABLE_DETECTION_FAILED",
                format!("Некорректный fixture JSON: {}", err),
            )
        })?;

        let mut regions = Vec::new();
        for (idx, item) in parsed.into_iter().enumerate() {
            let typ = item.get("type").and_then(|v| v.as_str()).unwrap_or_default();
            if typ != "table" {
                continue;
            }
            let Some(arr) = item.get("bbox").and_then(|v| v.as_array()) else {
                continue;
            };
            if arr.len() != 4 {
                continue;
            }
            let bbox = BBox {
                x0: arr[0].as_f64().unwrap_or(0.0) as f32,
                y0: arr[1].as_f64().unwrap_or(0.0) as f32,
                x1: arr[2].as_f64().unwrap_or(0.0) as f32,
                y1: arr[3].as_f64().unwrap_or(0.0) as f32,
            };
            regions.push(TableRegion {
                region_id: format!("table_fixture_{}", idx + 1),
                page_number: input.page_number,
                bbox,
                confidence: item
                    .get("confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.8) as f32,
                source: "fixture_scanned_table_detector".to_string(),
            });
        }

        context.push_stage(
            ConversionStageRecord::ok("scanned_table_detection", "fixture_scanned_table_detector")
                .with_meta("regions", regions.len().to_string()),
        );

        Ok(regions)
    }
}

#[derive(Debug, Default, Clone)]
pub struct DisabledScannedTableDetector;

#[async_trait]
impl ScannedTableDetector for DisabledScannedTableDetector {
    async fn detect_tables(
        &self,
        _input: TableDetectionInput,
        context: &mut ExtractionContext,
    ) -> Result<Vec<TableRegion>> {
        context.push_stage(
            ConversionStageRecord::warning("scanned_table_detection", "disabled_scanned_table_detector")
                .with_meta("regions", "0"),
        );
        Ok(Vec::new())
    }
}
