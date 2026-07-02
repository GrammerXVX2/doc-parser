use async_trait::async_trait;
use serde_json::json;

use crate::converters::traits::ExtractionContext;
use crate::models::backends::http::HttpModelBackendClient;
use crate::models::backends::http_schemas::{
    StructuredParseHttpRequest, StructuredParseHttpResponse,
};
use crate::models::backends::traits::{
    ModelBackend, ModelBackendHealth, StructuredDocumentParserBackend, StructuredParseInput,
    StructuredParseOutput,
};
use crate::models::config::ModelBackendConfig;

#[derive(Clone)]
pub struct DoclingStructuredParseHttpBackend {
    client: HttpModelBackendClient,
    config: ModelBackendConfig,
}

impl DoclingStructuredParseHttpBackend {
    pub fn new(config: ModelBackendConfig) -> Self {
        let base_url = config
            .backend_url()
            .unwrap_or_else(|| "http://127.0.0.1:8103".to_string());
        let timeout = config.timeout(300);
        Self {
            client: HttpModelBackendClient::new("docling", base_url, timeout),
            config,
        }
    }

    pub fn backend_url(&self) -> String {
        self.client.base_url.clone()
    }

    pub async fn parse_raw(
        &self,
        input: &StructuredParseInput,
        language: &str,
    ) -> anyhow::Result<StructuredParseHttpResponse> {
        let request = StructuredParseHttpRequest {
            document_id: input.document_id.clone(),
            input_path: input.input_path.clone(),
            format: detect_format(&input.input_path),
            language: language.to_string(),
            options: json!({}),
        };
        let path = self.config.endpoint_path("parse_path", "/v1/parse");
        self.client.post_json(&path, &request).await
    }
}

#[async_trait]
impl ModelBackend for DoclingStructuredParseHttpBackend {
    fn name(&self) -> &str {
        "docling"
    }

    fn kind(&self) -> &str {
        "structured_document_parse"
    }

    async fn health_check(&self) -> ModelBackendHealth {
        let health_path = self.config.endpoint_path("health_path", "/healthz");
        self.client.health_check(&health_path).await
    }
}

#[async_trait]
impl StructuredDocumentParserBackend for DoclingStructuredParseHttpBackend {
    async fn parse_document_structured(
        &self,
        input: StructuredParseInput,
        _context: &mut ExtractionContext,
    ) -> anyhow::Result<StructuredParseOutput> {
        let response = self.parse_raw(&input, "ru").await?;
        Ok(StructuredParseOutput {
            executed: true,
            metadata: json!({
                "backend": response.backend,
                "markdown": response.markdown,
                "text": response.text,
                "elements": response.elements,
                "tables": response.tables,
                "metadata": response.metadata,
                "confidence": response.confidence,
            }),
        })
    }
}

fn detect_format(path: &str) -> String {
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("unknown")
        .to_ascii_lowercase();

    if extension.is_empty() {
        "unknown".to_string()
    } else {
        extension
    }
}
