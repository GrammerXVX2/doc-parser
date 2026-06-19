use std::path::PathBuf;

use async_trait::async_trait;

use crate::converters::traits::{ConversionError, ConversionStageRecord, ExtractionContext, Result};
use crate::utils::geometry::BBox;

#[derive(Debug, Clone)]
pub struct FormulaDetectionInput {
    pub document_id: String,
    pub page_number: usize,
    pub page_image_path: Option<PathBuf>,
    pub page_width: f32,
    pub page_height: f32,
}

#[derive(Debug, Clone)]
pub struct FormulaRegion {
    pub region_id: String,
    pub page_number: usize,
    pub bbox: BBox,
    pub confidence: f32,
    pub source: String,
}

#[async_trait]
pub trait FormulaDetector: Send + Sync {
    async fn detect_formulas(
        &self,
        input: FormulaDetectionInput,
        context: &mut ExtractionContext,
    ) -> Result<Vec<FormulaRegion>>;
}

#[derive(Debug, Default, Clone)]
pub struct MockFormulaDetector;

#[async_trait]
impl FormulaDetector for MockFormulaDetector {
    async fn detect_formulas(
        &self,
        input: FormulaDetectionInput,
        context: &mut ExtractionContext,
    ) -> Result<Vec<FormulaRegion>> {
        let regions = vec![FormulaRegion {
            region_id: "formula_mock_1".to_string(),
            page_number: input.page_number,
            bbox: BBox {
                x0: input.page_width * 0.2,
                y0: input.page_height * 0.72,
                x1: input.page_width * 0.82,
                y1: input.page_height * 0.84,
            },
            confidence: 0.85,
            source: "mock_formula_detector".to_string(),
        }];

        context.push_stage(
            ConversionStageRecord::ok("formula_detection", "mock_formula_detector")
                .with_meta("regions", regions.len().to_string()),
        );
        Ok(regions)
    }
}

#[derive(Debug, Default, Clone)]
pub struct FixtureFormulaDetector;

#[async_trait]
impl FormulaDetector for FixtureFormulaDetector {
    async fn detect_formulas(
        &self,
        input: FormulaDetectionInput,
        context: &mut ExtractionContext,
    ) -> Result<Vec<FormulaRegion>> {
        let Some(path) = input.page_image_path else {
            return Err(ConversionError::new(
                "FORMULA_DETECTION_FAILED",
                "Для fixture formula detector отсутствует page_image_path.",
            ));
        };

        let fixture = if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            path.with_extension(format!("{}.layout.json", ext))
        } else {
            path.with_extension("layout.json")
        };

        if !fixture.exists() {
            return Err(ConversionError::new(
                "FORMULA_DETECTION_FAILED",
                format!("Fixture formula regions не найден: {}", fixture.to_string_lossy()),
            ));
        }

        let raw = std::fs::read_to_string(&fixture).map_err(|err| {
            ConversionError::new(
                "FORMULA_DETECTION_FAILED",
                format!("Не удалось прочитать formula fixture: {}", err),
            )
        })?;

        let parsed: Vec<serde_json::Value> = serde_json::from_str(&raw).map_err(|err| {
            ConversionError::new(
                "FORMULA_DETECTION_FAILED",
                format!("Некорректный formula fixture JSON: {}", err),
            )
        })?;

        let mut regions = Vec::new();
        for (idx, item) in parsed.into_iter().enumerate() {
            let typ = item.get("type").and_then(|v| v.as_str()).unwrap_or_default();
            if typ != "formula" {
                continue;
            }
            let Some(arr) = item.get("bbox").and_then(|v| v.as_array()) else {
                continue;
            };
            if arr.len() != 4 {
                continue;
            }
            regions.push(FormulaRegion {
                region_id: format!("formula_fixture_{}", idx + 1),
                page_number: input.page_number,
                bbox: BBox {
                    x0: arr[0].as_f64().unwrap_or(0.0) as f32,
                    y0: arr[1].as_f64().unwrap_or(0.0) as f32,
                    x1: arr[2].as_f64().unwrap_or(0.0) as f32,
                    y1: arr[3].as_f64().unwrap_or(0.0) as f32,
                },
                confidence: item
                    .get("confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.85) as f32,
                source: "fixture_formula_detector".to_string(),
            });
        }

        context.push_stage(
            ConversionStageRecord::ok("formula_detection", "fixture_formula_detector")
                .with_meta("regions", regions.len().to_string()),
        );

        Ok(regions)
    }
}

#[derive(Debug, Default, Clone)]
pub struct DisabledFormulaDetector;

#[async_trait]
impl FormulaDetector for DisabledFormulaDetector {
    async fn detect_formulas(
        &self,
        _input: FormulaDetectionInput,
        context: &mut ExtractionContext,
    ) -> Result<Vec<FormulaRegion>> {
        context.push_stage(
            ConversionStageRecord::warning("formula_detection", "disabled_formula_detector")
                .with_meta("regions", "0"),
        );
        Ok(Vec::new())
    }
}
