pub mod detector;
pub mod omml_latex;
pub mod placeholder;
pub mod recognizer;

use async_trait::async_trait;

use crate::converters::traits::{ExtractionContext, Result};
use crate::model::Element;

pub use detector::{
    DisabledFormulaDetector, FixtureFormulaDetector, FormulaDetectionInput, FormulaDetector,
    FormulaRegion, MockFormulaDetector,
};
pub use placeholder::create_formula_placeholder;
pub use recognizer::{
    DisabledFormulaRecognizer, FixtureFormulaRecognizer, FormulaRecognitionInput, FormulaRecognizer,
    MockFormulaRecognizer,
};

#[async_trait]
pub trait ElementEnricher: Send + Sync {
    async fn enrich(&self, element: &mut Element, context: &mut ExtractionContext) -> Result<()>;
}

#[derive(Debug, Default, Clone)]
pub struct ImageEnricher;

#[async_trait]
impl ElementEnricher for ImageEnricher {
    async fn enrich(&self, _element: &mut Element, _context: &mut ExtractionContext) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct ChartEnricher;

#[async_trait]
impl ElementEnricher for ChartEnricher {
    async fn enrich(&self, _element: &mut Element, _context: &mut ExtractionContext) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct FormulaEnricher;

#[async_trait]
impl ElementEnricher for FormulaEnricher {
    async fn enrich(&self, element: &mut Element, _context: &mut ExtractionContext) -> Result<()> {
        if element.extra.get("format").is_none() {
            element
                .extra
                .insert("format".to_string(), serde_json::json!("unknown"));
        }
        Ok(())
    }
}
