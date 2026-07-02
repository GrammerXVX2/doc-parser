use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::json;

use crate::converters::traits::ExtractionContext;
use crate::model::{Element, ElementType};

use super::traits::{
    ExtendedFormulaBackend, ExtendedFormulaInput, ExtendedOcrBackend, ExtendedOcrInput,
    ExtendedOcrOutput, ExtendedTableBackend, ExtendedTableInput, ModelBackend,
    ModelBackendHealth, StructuredDocumentParserBackend, StructuredParseInput,
    StructuredParseOutput, VlmDocumentBackend, VlmDocumentInput, VlmDocumentOutput,
};

macro_rules! define_mock_backend {
    ($name:ident, $kind:expr, $label:expr) => {
        #[derive(Debug, Clone, Default)]
        pub struct $name;

        #[async_trait]
        impl ModelBackend for $name {
            fn name(&self) -> &str {
                $label
            }

            fn kind(&self) -> &str {
                $kind
            }

            async fn health_check(&self) -> ModelBackendHealth {
                ModelBackendHealth {
                    available: true,
                    message: Some("mock backend is available".to_string()),
                    details: json!({
                        "backend": $label,
                        "deterministic": true,
                        "fixture_support": true
                    }),
                }
            }
        }
    };
}

define_mock_backend!(MockPaddleOcrV6Backend, "ocr", "mock_paddleocr_v6");
define_mock_backend!(MockSuryaOcrBackend, "ocr", "mock_surya_ocr");
define_mock_backend!(MockDoclingBackend, "structured_document_parse", "mock_docling");
define_mock_backend!(MockPaddleOcrVl16Backend, "vlm_document_parse", "mock_paddleocr_vl_1_6");
define_mock_backend!(MockQwen3VlBackend, "vlm", "mock_qwen3_vl");
define_mock_backend!(MockGraniteDocling258MBackend, "vlm_document_parse", "mock_granite_docling_258m");
define_mock_backend!(MockSuryaLayoutBackend, "layout", "mock_surya_layout");
define_mock_backend!(MockDoclingLayoutBackend, "layout", "mock_docling_layout");
define_mock_backend!(MockTableTransformerBackend, "table_structure", "mock_table_transformer");
define_mock_backend!(MockPix2TexBackend, "formula", "mock_pix2tex");
define_mock_backend!(MockGlinerBackend, "legal_ner", "mock_gliner");
define_mock_backend!(MockBgeM3Backend, "embedding", "mock_bge_m3");
define_mock_backend!(MockUserBgeM3Backend, "embedding", "mock_user_bge_m3");
define_mock_backend!(MockKrakenBackend, "historical_ocr", "mock_kraken");

#[async_trait]
impl ExtendedOcrBackend for MockPaddleOcrV6Backend {
    async fn run_ocr(
        &self,
        input: ExtendedOcrInput,
        _context: &mut ExtractionContext,
    ) -> anyhow::Result<ExtendedOcrOutput> {
        Ok(ExtendedOcrOutput {
            elements: vec![mock_text_element(input.page_number, "mock paddle ocr text")],
            confidence: Some(0.92),
            provenance: json!({
                "backend": self.name(),
                "fixture": "page_1.ocr.json"
            }),
        })
    }
}

#[async_trait]
impl ExtendedOcrBackend for MockSuryaOcrBackend {
    async fn run_ocr(
        &self,
        input: ExtendedOcrInput,
        _context: &mut ExtractionContext,
    ) -> anyhow::Result<ExtendedOcrOutput> {
        Ok(ExtendedOcrOutput {
            elements: vec![mock_text_element(input.page_number, "mock surya ocr text")],
            confidence: Some(0.88),
            provenance: json!({
                "backend": self.name(),
                "fixture": "page_1.ocr.json"
            }),
        })
    }
}

#[async_trait]
impl ExtendedOcrBackend for MockKrakenBackend {
    async fn run_ocr(
        &self,
        input: ExtendedOcrInput,
        _context: &mut ExtractionContext,
    ) -> anyhow::Result<ExtendedOcrOutput> {
        Ok(ExtendedOcrOutput {
            elements: vec![mock_text_element(input.page_number, "mock kraken historical text")],
            confidence: Some(0.81),
            provenance: json!({
                "backend": self.name(),
                "fixture": "page_1.ocr.json"
            }),
        })
    }
}

#[async_trait]
impl StructuredDocumentParserBackend for MockDoclingBackend {
    async fn parse_document_structured(
        &self,
        input: StructuredParseInput,
        _context: &mut ExtractionContext,
    ) -> anyhow::Result<StructuredParseOutput> {
        Ok(StructuredParseOutput {
            executed: true,
            metadata: json!({
                "backend": self.name(),
                "document_id": input.document_id,
                "fixture": "input.model_routes.json"
            }),
        })
    }
}

#[async_trait]
impl StructuredDocumentParserBackend for MockPaddleOcrVl16Backend {
    async fn parse_document_structured(
        &self,
        input: StructuredParseInput,
        _context: &mut ExtractionContext,
    ) -> anyhow::Result<StructuredParseOutput> {
        Ok(StructuredParseOutput {
            executed: true,
            metadata: json!({
                "backend": self.name(),
                "document_id": input.document_id
            }),
        })
    }
}

#[async_trait]
impl VlmDocumentBackend for MockQwen3VlBackend {
    async fn analyze_page_or_document(
        &self,
        input: VlmDocumentInput,
        _context: &mut ExtractionContext,
    ) -> anyhow::Result<VlmDocumentOutput> {
        Ok(VlmDocumentOutput {
            executed: true,
            summary: Some(format!("mock vlm summary for {}", input.document_id)),
            metadata: json!({
                "backend": self.name(),
                "fixture": "input.model_routes.json"
            }),
        })
    }
}

#[async_trait]
impl VlmDocumentBackend for MockGraniteDocling258MBackend {
    async fn analyze_page_or_document(
        &self,
        input: VlmDocumentInput,
        _context: &mut ExtractionContext,
    ) -> anyhow::Result<VlmDocumentOutput> {
        Ok(VlmDocumentOutput {
            executed: true,
            summary: Some(format!("mock granite summary for {}", input.document_id)),
            metadata: json!({
                "backend": self.name()
            }),
        })
    }
}

#[async_trait]
impl ExtendedTableBackend for MockTableTransformerBackend {
    async fn recognize_table(
        &self,
        input: ExtendedTableInput,
        _context: &mut ExtractionContext,
    ) -> anyhow::Result<Element> {
        Ok(mock_table_element(input.page_number))
    }
}

#[async_trait]
impl ExtendedFormulaBackend for MockPix2TexBackend {
    async fn recognize_formula(
        &self,
        input: ExtendedFormulaInput,
        _context: &mut ExtractionContext,
    ) -> anyhow::Result<Element> {
        Ok(mock_formula_element(input.page_number))
    }
}

fn mock_text_element(page_number: usize, text: &str) -> Element {
    Element {
        element_id: format!("p{page_number}_ocr_1"),
        element_type: ElementType::TextOcr,
        tag: None,
        role: None,
        reading_order: Some(1),
        global_order: Some(1),
        bbox: None,
        polygon: None,
        content: json!({"text": text}),
        style: json!({}),
        provenance: json!({
            "backend": "mock",
            "fixture": "page_1.ocr.json"
        }),
        confidence: json!({"overall": 0.9}),
        warnings: vec![],
        extra: HashMap::new(),
    }
}

fn mock_table_element(page_number: usize) -> Element {
    let mut extra = HashMap::new();
    extra.insert("rows".to_string(), json!(2));
    extra.insert("columns".to_string(), json!(2));

    Element {
        element_id: format!("p{page_number}_table_1"),
        element_type: ElementType::Table,
        tag: Some("table".to_string()),
        role: None,
        reading_order: Some(1),
        global_order: Some(1),
        bbox: None,
        polygon: None,
        content: json!({"text": "mock table", "cells": []}),
        style: json!({}),
        provenance: json!({"fixture": "page_1.table.json"}),
        confidence: json!({"overall": 0.82}),
        warnings: vec![],
        extra,
    }
}

fn mock_formula_element(page_number: usize) -> Element {
    Element {
        element_id: format!("p{page_number}_formula_1"),
        element_type: ElementType::Formula,
        tag: None,
        role: None,
        reading_order: Some(1),
        global_order: Some(1),
        bbox: None,
        polygon: None,
        content: json!({"latex": "E=mc^2"}),
        style: json!({}),
        provenance: json!({"fixture": "page_1.formula.json"}),
        confidence: json!({"overall": 0.8}),
        warnings: vec![],
        extra: HashMap::new(),
    }
}
