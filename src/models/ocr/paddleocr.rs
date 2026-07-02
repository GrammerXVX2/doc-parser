use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::json;

use crate::converters::traits::ExtractionContext;
use crate::model::{Element, ElementType};
use crate::models::backends::http::HttpModelBackendClient;
use crate::models::backends::http_schemas::{OcrHttpRequest, OcrHttpResponse};
use crate::models::backends::traits::{
    ExtendedOcrBackend, ExtendedOcrInput, ExtendedOcrOutput, ModelBackend, ModelBackendHealth,
};
use crate::models::config::ModelBackendConfig;

#[derive(Clone)]
pub struct PaddleOcrV6HttpBackend {
    client: HttpModelBackendClient,
    config: ModelBackendConfig,
}

impl PaddleOcrV6HttpBackend {
    pub fn new(config: ModelBackendConfig) -> Self {
        let base_url = config
            .backend_url()
            .unwrap_or_else(|| "http://127.0.0.1:8101".to_string());
        let timeout = config.timeout(120);
        Self {
            client: HttpModelBackendClient::new("paddleocr_ppocrv6_medium", base_url, timeout),
            config,
        }
    }

    pub fn backend_url(&self) -> String {
        self.client.base_url.clone()
    }
}

#[async_trait]
impl ModelBackend for PaddleOcrV6HttpBackend {
    fn name(&self) -> &str {
        "paddleocr_ppocrv6_medium"
    }

    fn kind(&self) -> &str {
        "ocr"
    }

    async fn health_check(&self) -> ModelBackendHealth {
        let health_path = self.config.endpoint_path("health_path", "/healthz");
        self.client.health_check(&health_path).await
    }
}

#[async_trait]
impl ExtendedOcrBackend for PaddleOcrV6HttpBackend {
    async fn run_ocr(
        &self,
        input: ExtendedOcrInput,
        _context: &mut ExtractionContext,
    ) -> anyhow::Result<ExtendedOcrOutput> {
        let image_path = input.image_path.clone().ok_or_else(|| {
            anyhow::anyhow!("MODEL_BACKEND_RESPONSE_INVALID: OCR input missing image_path")
        })?;
        let request = OcrHttpRequest {
            document_id: input.document_id.clone(),
            page_number: input.page_number as u32,
            image_path,
            languages: input.languages.clone(),
            options: json!({}),
        };
        let ocr_path = self.config.endpoint_path("ocr_path", "/v1/ocr");
        let response: OcrHttpResponse = self.client.post_json(&ocr_path, &request).await?;

        let elements = response
            .regions
            .iter()
            .enumerate()
            .map(|(idx, region)| {
                let mut extra = HashMap::new();
                if let Some(language) = &region.language {
                    extra.insert("language".to_string(), json!(language));
                }

                Element {
                    element_id: format!("p{}_paddle_ocr_{}", input.page_number, idx + 1),
                    element_type: ElementType::TextOcr,
                    tag: None,
                    role: None,
                    reading_order: Some((idx + 1) as u32),
                    global_order: Some((idx + 1) as u32),
                    bbox: Some(region.bbox),
                    polygon: None,
                    content: json!({"text": region.text}),
                    style: json!({}),
                    provenance: json!({
                        "method": "ocr_http",
                        "tool": "paddleocr_ppocrv6_medium",
                        "stage": "paddleocr_http_ocr",
                        "backend_url": self.client.base_url,
                        "response_backend": response.backend,
                    }),
                    confidence: json!({"overall": region.confidence}),
                    warnings: vec![],
                    extra,
                }
            })
            .collect::<Vec<_>>();

        Ok(ExtendedOcrOutput {
            elements,
            confidence: response.confidence,
            provenance: json!({
                "backend": response.backend,
                "backend_type": "http",
                "url": self.client.base_url,
                "metadata": response.metadata,
            }),
        })
    }
}
