use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use tracing::info;
use uuid::Uuid;

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

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub path: String,
}
