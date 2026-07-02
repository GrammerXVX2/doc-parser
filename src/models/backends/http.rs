use std::time::Duration;

use reqwest::StatusCode;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;

use super::traits::ModelBackendHealth;

#[derive(Clone)]
pub struct HttpModelBackendClient {
    pub name: String,
    pub base_url: String,
    pub timeout: Duration,
    client: reqwest::Client,
}

impl HttpModelBackendClient {
    pub fn new(name: impl Into<String>, base_url: impl Into<String>, timeout: Duration) -> Self {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            name: name.into(),
            base_url,
            timeout,
            client,
        }
    }

    pub async fn health_check(&self, health_path: &str) -> ModelBackendHealth {
        let url = self.url(health_path);
        let response = self.client.get(&url).send().await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let details = resp
                    .json::<serde_json::Value>()
                    .await
                    .unwrap_or_else(|_| json!({"status": "ok"}));
                ModelBackendHealth {
                    available: true,
                    message: Some("http service healthy".to_string()),
                    details: json!({
                        "url": url,
                        "response": details,
                    }),
                }
            }
            Ok(resp) => ModelBackendHealth {
                available: false,
                message: Some(format!("health endpoint returned HTTP {}", resp.status().as_u16())),
                details: json!({
                    "url": url,
                    "status": resp.status().as_u16(),
                }),
            },
            Err(err) => {
                let message = if err.is_timeout() {
                    format!("MODEL_BACKEND_TIMEOUT: {} health check timeout", self.name)
                } else {
                    format!("MODEL_BACKEND_HTTP_ERROR: {} health check failed: {}", self.name, err)
                };
                ModelBackendHealth {
                    available: false,
                    message: Some(message),
                    details: json!({
                        "url": url,
                        "error": err.to_string(),
                    }),
                }
            }
        }
    }

    pub async fn post_json<TReq, TResp>(
        &self,
        path: &str,
        payload: &TReq,
    ) -> anyhow::Result<TResp>
    where
        TReq: Serialize + ?Sized,
        TResp: DeserializeOwned,
    {
        let url = self.url(path);

        let response = self
            .client
            .post(&url)
            .json(payload)
            .send()
            .await
            .map_err(|err| {
                if err.is_timeout() {
                    anyhow::anyhow!("MODEL_BACKEND_TIMEOUT: backend '{}' request timeout on {}", self.name, url)
                } else {
                    anyhow::anyhow!("MODEL_BACKEND_HTTP_ERROR: backend '{}' request failed on {}: {}", self.name, url, err)
                }
            })?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(http_status_error(&self.name, &url, response.status())));
        }

        response.json::<TResp>().await.map_err(|err| {
            anyhow::anyhow!(
                "MODEL_BACKEND_RESPONSE_INVALID: backend '{}' returned invalid JSON on {}: {}",
                self.name,
                url,
                err
            )
        })
    }

    fn url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            return path.to_string();
        }

        let suffix = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        };
        format!("{}{}", self.base_url, suffix)
    }
}

fn http_status_error(name: &str, url: &str, status: StatusCode) -> String {
    format!(
        "MODEL_BACKEND_HTTP_ERROR: backend '{}' returned HTTP {} on {}",
        name,
        status.as_u16(),
        url
    )
}
