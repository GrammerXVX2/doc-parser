use async_trait::async_trait;
use serde_json::json;

use crate::layout::{LayoutRegion, LayoutRegionType, LayoutSource};
use crate::models::backends::http::HttpModelBackendClient;
use crate::models::backends::http_schemas::{LayoutHttpRequest, LayoutHttpResponse};
use crate::models::backends::traits::{ModelBackend, ModelBackendHealth};
use crate::models::config::ModelBackendConfig;
use crate::utils::geometry::BBox;

#[derive(Clone)]
pub struct SuryaLayoutHttpBackend {
    client: HttpModelBackendClient,
    config: ModelBackendConfig,
}

impl SuryaLayoutHttpBackend {
    pub fn new(config: ModelBackendConfig) -> Self {
        let base_url = config
            .backend_url()
            .unwrap_or_else(|| "http://127.0.0.1:8102".to_string());
        let timeout = config.timeout(120);
        Self {
            client: HttpModelBackendClient::new("surya_layout", base_url, timeout),
            config,
        }
    }

    pub async fn detect_layout(&self, request: LayoutHttpRequest) -> anyhow::Result<LayoutHttpResponse> {
        let path = self.config.endpoint_path("layout_path", "/v1/layout");
        self.client.post_json(&path, &request).await
    }

    pub fn to_layout_regions(&self, page_number: usize, response: &LayoutHttpResponse) -> Vec<LayoutRegion> {
        response
            .regions
            .iter()
            .enumerate()
            .map(|(idx, region)| LayoutRegion {
                region_id: format!("surya_layout_{}", idx + 1),
                page_number,
                region_type: LayoutRegionType::from_str(&region.region_type),
                bbox: BBox::from_array(region.bbox),
                polygon: None,
                confidence: region.confidence,
                reading_order: Some(idx + 1),
                source: LayoutSource::Model,
            })
            .collect()
    }

    pub fn backend_url(&self) -> String {
        self.client.base_url.clone()
    }
}

#[async_trait]
impl ModelBackend for SuryaLayoutHttpBackend {
    fn name(&self) -> &str {
        "surya_layout"
    }

    fn kind(&self) -> &str {
        "layout"
    }

    async fn health_check(&self) -> ModelBackendHealth {
        let health_path = self.config.endpoint_path("health_path", "/healthz");
        self.client.health_check(&health_path).await
    }
}

#[derive(Clone)]
pub struct DoclingLayoutHttpBackend {
    client: HttpModelBackendClient,
    config: ModelBackendConfig,
}

impl DoclingLayoutHttpBackend {
    pub fn new(config: ModelBackendConfig) -> Self {
        let base_url = config
            .backend_url()
            .unwrap_or_else(|| "http://127.0.0.1:8103".to_string());
        let timeout = config.timeout(300);
        Self {
            client: HttpModelBackendClient::new("docling_layout", base_url, timeout),
            config,
        }
    }

    pub async fn detect_layout(&self, request: LayoutHttpRequest) -> anyhow::Result<LayoutHttpResponse> {
        let path = self.config.endpoint_path("layout_path", "/v1/layout");
        self.client.post_json(&path, &request).await
    }

    pub fn to_layout_regions(&self, page_number: usize, response: &LayoutHttpResponse) -> Vec<LayoutRegion> {
        response
            .regions
            .iter()
            .enumerate()
            .map(|(idx, region)| LayoutRegion {
                region_id: format!("docling_layout_{}", idx + 1),
                page_number,
                region_type: LayoutRegionType::from_str(&region.region_type),
                bbox: BBox::from_array(region.bbox),
                polygon: None,
                confidence: region.confidence,
                reading_order: Some(idx + 1),
                source: LayoutSource::Model,
            })
            .collect()
    }

    pub fn backend_url(&self) -> String {
        self.client.base_url.clone()
    }
}

#[async_trait]
impl ModelBackend for DoclingLayoutHttpBackend {
    fn name(&self) -> &str {
        "docling_layout"
    }

    fn kind(&self) -> &str {
        "layout"
    }

    async fn health_check(&self) -> ModelBackendHealth {
        let health_path = self.config.endpoint_path("health_path", "/healthz");
        self.client.health_check(&health_path).await
    }
}

pub fn layout_request(
    document_id: &str,
    page_number: u32,
    image_path: Option<String>,
    width: Option<f32>,
    height: Option<f32>,
) -> LayoutHttpRequest {
    LayoutHttpRequest {
        document_id: document_id.to_string(),
        page_number,
        image_path,
        width,
        height,
        options: json!({}),
    }
}
