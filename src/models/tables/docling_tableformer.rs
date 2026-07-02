use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::json;

use crate::converters::traits::ExtractionContext;
use crate::model::{Element, ElementType};
use crate::models::backends::http::HttpModelBackendClient;
use crate::models::backends::http_schemas::{LayoutHttpRequest, LayoutHttpResponse};
use crate::models::backends::traits::{
    ExtendedTableBackend, ExtendedTableInput, ModelBackend, ModelBackendHealth,
};
use crate::models::config::ModelBackendConfig;

#[derive(Clone)]
pub struct DoclingTableFormerHttpBackend {
    client: HttpModelBackendClient,
    config: ModelBackendConfig,
}

impl DoclingTableFormerHttpBackend {
    pub fn new(config: ModelBackendConfig) -> Self {
        let base_url = config
            .backend_url()
            .unwrap_or_else(|| "http://127.0.0.1:8103".to_string());
        let timeout = config.timeout(300);
        Self {
            client: HttpModelBackendClient::new("docling_tableformer", base_url, timeout),
            config,
        }
    }

    pub fn backend_url(&self) -> String {
        self.client.base_url.clone()
    }

    pub async fn detect_tables(&self, request: LayoutHttpRequest) -> anyhow::Result<LayoutHttpResponse> {
        let path = self.config.endpoint_path("table_path", "/v1/tables");
        self.client.post_json(&path, &request).await
    }
}

#[async_trait]
impl ModelBackend for DoclingTableFormerHttpBackend {
    fn name(&self) -> &str {
        "docling_tableformer"
    }

    fn kind(&self) -> &str {
        "table_structure"
    }

    async fn health_check(&self) -> ModelBackendHealth {
        let health_path = self.config.endpoint_path("health_path", "/healthz");
        self.client.health_check(&health_path).await
    }
}

#[async_trait]
impl ExtendedTableBackend for DoclingTableFormerHttpBackend {
    async fn recognize_table(
        &self,
        input: ExtendedTableInput,
        _context: &mut ExtractionContext,
    ) -> anyhow::Result<Element> {
        let request = LayoutHttpRequest {
            document_id: input.document_id,
            page_number: input.page_number as u32,
            image_path: None,
            width: None,
            height: None,
            options: input.region_hint.unwrap_or_else(|| json!({})),
        };
        let response = self.detect_tables(request).await?;
        let region = response
            .regions
            .first()
            .ok_or_else(|| anyhow::anyhow!("MODEL_BACKEND_RESPONSE_INVALID: docling table regions are empty"))?;

        Ok(Element {
            element_id: format!("p{}_docling_table_http_1", input.page_number),
            element_type: ElementType::Table,
            tag: Some("table".to_string()),
            role: Some("table_structure".to_string()),
            reading_order: Some(1),
            global_order: Some(1),
            bbox: Some(region.bbox),
            polygon: None,
            content: json!({"text": "[Docling table region]", "cells": []}),
            style: json!({}),
            provenance: json!({
                "method": "table_http",
                "tool": "docling_tableformer",
                "stage": "docling_http_table",
                "backend_url": self.client.base_url,
            }),
            confidence: json!({"overall": region.confidence}),
            warnings: vec![],
            extra: HashMap::new(),
        })
    }
}
