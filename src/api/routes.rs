use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{get, post};
use tower_http::cors::CorsLayer;

use crate::api::handlers;
use crate::api::middleware::{auth_middleware, request_context_middleware};
use crate::api::server::AppState;

pub fn build_router(state: AppState) -> Router {
    let public_routes = Router::new()
        .route("/healthz", get(handlers::healthz))
        .route("/readyz", get(handlers::readyz))
        .route("/metrics", get(handlers::metrics));

    let protected_routes = Router::new()
        .route("/v1/system/performance", get(handlers::system_performance))
        .route("/v1/documents", post(handlers::upload_document))
        .route("/v1/jobs/:job_id", get(handlers::get_job_status))
        .route("/v1/documents/:document_id/model", get(handlers::get_document_model))
        .route("/v1/documents/:document_id/markdown", get(handlers::get_document_markdown))
        .route("/v1/documents/:document_id/text", get(handlers::get_document_text))
        .route(
            "/v1/documents/:document_id/assets/:asset_id",
            get(handlers::get_document_asset),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(DefaultBodyLimit::disable())
        .layer(CorsLayer::permissive())
        .layer(middleware::from_fn(request_context_middleware))
        .with_state(state)
}
