use std::collections::HashMap;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorPayload {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
    #[serde(default)]
    pub details: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: StatusCode,
    pub payload: ApiErrorPayload,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &str, message: &str, recoverable: bool) -> Self {
        Self {
            status,
            payload: ApiErrorPayload {
                code: code.to_string(),
                message: message.to_string(),
                recoverable,
                details: HashMap::new(),
            },
        }
    }

    pub fn with_detail(mut self, key: &str, value: serde_json::Value) -> Self {
        self.payload.details.insert(key.to_string(), value);
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = json!({
            "error": self.payload,
        });
        (self.status, axum::Json(body)).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
