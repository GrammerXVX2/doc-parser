use std::time::Duration;

use anyhow::{Context, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TritonInferRequest {
    #[serde(default)]
    pub inputs: Value,
    #[serde(default)]
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TritonInferResponse {
    #[serde(default)]
    pub raw: Value,
}

#[derive(Debug, Clone)]
pub struct TritonClient {
    pub url: String,
    http: reqwest::blocking::Client,
}

impl TritonClient {
    pub fn new(url: impl Into<String>) -> anyhow::Result<Self> {
        let url = url.into();
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_millis(800))
            .build()
            .context("TRITON_UNAVAILABLE: failed to initialize Triton HTTP client")?;

        Ok(Self { url, http })
    }

    pub fn ensure_ready(&self) -> anyhow::Result<()> {
        if self.url.trim().is_empty() {
            return Err(anyhow!(
                "TRITON_NOT_CONFIGURED: Triton URL is empty in configuration"
            ));
        }

        let endpoint = format!("{}/v2/health/ready", self.url.trim_end_matches('/'));
        let response = self
            .http
            .get(&endpoint)
            .send()
            .map_err(|err| anyhow!("TRITON_UNAVAILABLE: {}", err))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "TRITON_UNAVAILABLE: readiness check returned {}",
                response.status()
            ));
        }

        Ok(())
    }

    pub fn infer(
        &self,
        model_name: &str,
        request: &TritonInferRequest,
    ) -> anyhow::Result<TritonInferResponse> {
        if self.url.trim().is_empty() {
            return Err(anyhow!(
                "TRITON_NOT_CONFIGURED: Triton URL is empty in configuration"
            ));
        }
        if model_name.trim().is_empty() {
            return Err(anyhow!(
                "TRITON_MODEL_UNAVAILABLE: model name is empty"
            ));
        }

        let endpoint = format!(
            "{}/v2/models/{}/infer",
            self.url.trim_end_matches('/'),
            model_name
        );
        let response = self
            .http
            .post(&endpoint)
            .json(request)
            .send()
            .map_err(|err| anyhow!("TRITON_REQUEST_FAILED: {}", err))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(anyhow!(
                "TRITON_MODEL_UNAVAILABLE: model '{}' was not found on Triton",
                model_name
            ));
        }

        if !response.status().is_success() {
            return Err(anyhow!(
                "TRITON_REQUEST_FAILED: Triton returned status {}",
                response.status()
            ));
        }

        let raw: Value = response
            .json()
            .context("TRITON_REQUEST_FAILED: failed to decode Triton response")?;

        Ok(TritonInferResponse { raw })
    }
}
