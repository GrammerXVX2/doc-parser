use async_trait::async_trait;

use crate::converters::traits::{ConversionError, ConversionStageRecord, ExtractionContext, Result};
use crate::formulas::detector::FormulaRegion;
use crate::formulas::placeholder::create_formula_placeholder;
use crate::model::Element;

#[derive(Debug, Clone)]
pub struct FormulaRecognitionInput {
    pub document_id: String,
    pub formula_region: FormulaRegion,
}

#[async_trait]
pub trait FormulaRecognizer: Send + Sync {
    async fn recognize_formula(
        &self,
        input: FormulaRecognitionInput,
        context: &mut ExtractionContext,
    ) -> Result<Element>;
}

#[derive(Debug, Default, Clone)]
pub struct MockFormulaRecognizer;

#[async_trait]
impl FormulaRecognizer for MockFormulaRecognizer {
    async fn recognize_formula(
        &self,
        input: FormulaRecognitionInput,
        context: &mut ExtractionContext,
    ) -> Result<Element> {
        context.push_stage(
            ConversionStageRecord::ok("formula_recognition", "mock_formula_recognizer")
                .with_meta("region_id", input.formula_region.region_id.clone()),
        );

        Ok(create_formula_placeholder(
            input.formula_region.page_number,
            &input.formula_region.region_id,
            input.formula_region.bbox,
            input.formula_region.confidence,
            "mock_formula_recognizer",
        ))
    }
}

#[derive(Debug, Default, Clone)]
pub struct FixtureFormulaRecognizer;

#[async_trait]
impl FormulaRecognizer for FixtureFormulaRecognizer {
    async fn recognize_formula(
        &self,
        input: FormulaRecognitionInput,
        context: &mut ExtractionContext,
    ) -> Result<Element> {
        context.push_stage(
            ConversionStageRecord::ok("formula_recognition", "fixture_formula_recognizer")
                .with_meta("region_id", input.formula_region.region_id.clone()),
        );

        Ok(create_formula_placeholder(
            input.formula_region.page_number,
            &input.formula_region.region_id,
            input.formula_region.bbox,
            input.formula_region.confidence,
            "fixture_formula_recognizer",
        ))
    }
}

#[derive(Debug, Default, Clone)]
pub struct DisabledFormulaRecognizer;

#[async_trait]
impl FormulaRecognizer for DisabledFormulaRecognizer {
    async fn recognize_formula(
        &self,
        input: FormulaRecognitionInput,
        context: &mut ExtractionContext,
    ) -> Result<Element> {
        context.push_stage(
            ConversionStageRecord::warning("formula_recognition", "disabled_formula_recognizer")
                .with_meta("region_id", input.formula_region.region_id.clone()),
        );

        Err(ConversionError::new(
            "FORMULA_RECOGNITION_FAILED",
            "Распознавание формул отключено для текущего backend.",
        ))
    }
}
