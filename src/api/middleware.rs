use std::time::Instant;

use axum::extract::State;
use axum::extract::Request;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use tracing::info;
use uuid::Uuid;

use crate::api::errors::ApiError;
use crate::api::server::AppState;

pub async fn request_context_middleware(mut request: Request, next: Next) -> Response {
    let request_id = Uuid::new_v4().simple().to_string();
    let path = request.uri().path().to_string();
    let method = request.method().to_string();
    let context = RequestContext {
        request_id,
        path,
    };
    request
        .extensions_mut()
        .insert(context.clone());

    let started = Instant::now();
    let response = next.run(request).await;
    let status = response.status().as_u16();
    let duration_ms = started.elapsed().as_millis();

    info!(
        request_id = %context.request_id,
        route = %context.path,
        method = %method,
        status = status,
        duration_ms = duration_ms,
        "api request handled"
    );
    response
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    if !state.profile.auth.enabled {
        return next.run(request).await;
    }

    let expected = std::env::var(&state.profile.auth.dev_token_env)
        .ok()
        .filter(|value| !value.trim().is_empty());

    let Some(expected) = expected else {
        return ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "AUTH_TOKEN_NOT_CONFIGURED",
            "Не задан токен авторизации сервиса (env).",
            false,
        )
        .into_response();
    };

    let provided = bearer_token(request.headers());
    if provided != Some(expected.as_str()) {
        return ApiError::new(
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED",
            "Необходим корректный Authorization Bearer token.",
            false,
        )
        .into_response();
    }

    next.run(request).await
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(axum::http::header::AUTHORIZATION)?;
    let value = value.to_str().ok()?;
    value.strip_prefix("Bearer ")
}

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub path: String,
}
