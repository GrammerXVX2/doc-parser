use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::converters::traits::ExtractionContext;
use crate::model::Element;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelBackendHealth {
    pub available: bool,
    pub message: Option<String>,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtendedOcrInput {
    pub document_id: String,
    pub page_number: usize,
    pub image_path: Option<String>,
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtendedOcrOutput {
    pub elements: Vec<Element>,
    pub confidence: Option<f32>,
    pub provenance: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuredParseInput {
    pub document_id: String,
    pub input_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuredParseOutput {
    pub executed: bool,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VlmDocumentInput {
    pub document_id: String,
    pub page_number: Option<usize>,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VlmDocumentOutput {
    pub executed: bool,
    pub summary: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtendedTableInput {
    pub document_id: String,
    pub page_number: usize,
    pub region_hint: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtendedFormulaInput {
    pub document_id: String,
    pub page_number: usize,
    pub region_hint: Option<Value>,
}

#[async_trait]
pub trait ModelBackend: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> &str;
    async fn health_check(&self) -> ModelBackendHealth;
}

#[async_trait]
pub trait ExtendedOcrBackend: ModelBackend {
    async fn run_ocr(
        &self,
        input: ExtendedOcrInput,
        context: &mut ExtractionContext,
    ) -> anyhow::Result<ExtendedOcrOutput>;
}

#[async_trait]
pub trait StructuredDocumentParserBackend: ModelBackend {
    async fn parse_document_structured(
        &self,
        input: StructuredParseInput,
        context: &mut ExtractionContext,
    ) -> anyhow::Result<StructuredParseOutput>;
}

#[async_trait]
pub trait VlmDocumentBackend: ModelBackend {
    async fn analyze_page_or_document(
        &self,
        input: VlmDocumentInput,
        context: &mut ExtractionContext,
    ) -> anyhow::Result<VlmDocumentOutput>;
}

#[async_trait]
pub trait ExtendedTableBackend: ModelBackend {
    async fn recognize_table(
        &self,
        input: ExtendedTableInput,
        context: &mut ExtractionContext,
    ) -> anyhow::Result<Element>;
}

#[async_trait]
pub trait ExtendedFormulaBackend: ModelBackend {
    async fn recognize_formula(
        &self,
        input: ExtendedFormulaInput,
        context: &mut ExtractionContext,
    ) -> anyhow::Result<Element>;
}
