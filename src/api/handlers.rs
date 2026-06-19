use std::collections::HashMap;
use std::path::PathBuf;

use axum::body::Body;
use axum::extract::{Multipart, Path, State};
use axum::extract::multipart::MultipartRejection;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, response::Response};
use chrono::Utc;
use uuid::Uuid;

use crate::api::errors::{ApiError, ApiResult};
use crate::api::responses::{
    HealthResponse, JobStatusResponse, ReadyResponse, SystemPerformanceResponse,
    UploadAcceptedResponse,
};
use crate::api::server::AppState;
use crate::jobs::{Job, JobStatus, ProcessingOptions};
use crate::security::{safe_join, validate_upload};
use crate::storage::DocumentStore;

pub async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

pub async fn readyz(State(state): State<AppState>) -> Json<ReadyResponse> {
    if !state.config_valid {
        return Json(ReadyResponse {
            status: "failed".to_string(),
            queue_capacity_remaining: state.queue_remaining(),
            workers: state.workers,
            message: Some("Конфигурация сервиса невалидна.".to_string()),
        });
    }

    if !state.storage_ready {
        return Json(ReadyResponse {
            status: "failed".to_string(),
            queue_capacity_remaining: state.queue_remaining(),
            workers: state.workers,
            message: Some("Хранилище недоступно для записи.".to_string()),
        });
    }

    if !state.model_registry_ready {
        return Json(ReadyResponse {
            status: "degraded".to_string(),
            queue_capacity_remaining: state.queue_remaining(),
            workers: state.workers,
            message: Some("Model registry не готов к обработке запросов.".to_string()),
        });
    }

    if state.queue.is_closed() {
        return Json(ReadyResponse {
            status: "failed".to_string(),
            queue_capacity_remaining: 0,
            workers: state.workers,
            message: Some("Очередь обработки недоступна.".to_string()),
        });
    }

    let remaining = state.queue_remaining();
    if remaining == 0 {
        return Json(ReadyResponse {
            status: "degraded".to_string(),
            queue_capacity_remaining: 0,
            workers: state.workers,
            message: Some("Очередь обработки заполнена.".to_string()),
        });
    }

    Json(ReadyResponse {
        status: "ok".to_string(),
        queue_capacity_remaining: remaining,
        workers: state.workers,
        message: None,
    })
}

pub async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    crate::observability::prometheus::metrics_handler(State(state)).await
}

pub async fn system_performance(State(state): State<AppState>) -> Json<SystemPerformanceResponse> {
    Json(SystemPerformanceResponse {
        provider: state.performance_provider.clone(),
        ocr_backend: state.ocr_backend.clone(),
        batching_enabled: state.batching_enabled,
        warmup_completed: state.warmup_completed,
    })
}

pub async fn upload_document(
    State(state): State<AppState>,
    multipart: Result<Multipart, MultipartRejection>,
) -> ApiResult<Json<UploadAcceptedResponse>> {
    if state.queue_remaining() == 0 {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "QUEUE_FULL",
            "Очередь обработки заполнена.",
            true,
        ));
    }

    let mut multipart = multipart.map_err(|err| {
        let text = err.to_string().to_ascii_lowercase();
        if text.contains("too large") || text.contains("body") || text.contains("limit") {
            ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "DOCUMENT_TOO_LARGE",
                "Размер документа превышает допустимый лимит.",
                false,
            )
        } else {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_MULTIPART",
                "Некорректный multipart-запрос.",
                false,
            )
        }
    })?;

    let mut file_name = None::<String>;
    let mut file_bytes = None::<Vec<u8>>;
    let mut options = ProcessingOptions::default();

    while let Some(field) = multipart.next_field().await.map_err(|err| {
        if err.status() == StatusCode::PAYLOAD_TOO_LARGE {
            ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "DOCUMENT_TOO_LARGE",
                "Размер документа превышает допустимый лимит.",
                false,
            )
        } else {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_MULTIPART",
                "Некорректный multipart-запрос.",
                false,
            )
        }
    })? {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "file" => {
                file_name = field.file_name().map(|v| v.to_string());
                file_bytes = Some(field.bytes().await.map_err(|err| {
                    if err.status() == StatusCode::PAYLOAD_TOO_LARGE {
                        ApiError::new(
                            StatusCode::PAYLOAD_TOO_LARGE,
                            "DOCUMENT_TOO_LARGE",
                            "Размер документа превышает допустимый лимит.",
                            false,
                        )
                    } else {
                        ApiError::new(
                            StatusCode::BAD_REQUEST,
                            "INVALID_UPLOAD",
                            "Не удалось прочитать загруженный файл.",
                            false,
                        )
                    }
                })?
                .to_vec());
            }
            "language" => {
                options.language = Some(field.text().await.unwrap_or_default());
            }
            "languages" => {
                let text = field.text().await.unwrap_or_default();
                let values = text
                    .split(',')
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>();
                if !values.is_empty() {
                    options.languages = Some(values);
                }
            }
            "ocr_backend" => options.ocr_backend = Some(field.text().await.unwrap_or_default()),
            "extract_tables" => {
                options.extract_tables = Some(matches!(
                    field.text().await.unwrap_or_default().to_ascii_lowercase().as_str(),
                    "true" | "1" | "yes"
                ));
            }
            "table_chunks" => {
                options.table_chunks = Some(matches!(
                    field.text().await.unwrap_or_default().to_ascii_lowercase().as_str(),
                    "true" | "1" | "yes"
                ));
            }
            "layout_backend" => options.layout_backend = Some(field.text().await.unwrap_or_default()),
            _ => {}
        }
    }

    let filename = file_name.ok_or_else(|| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "FILE_FIELD_MISSING",
            "Не найден обязательный multipart field 'file'.",
            false,
        )
    })?;
    let bytes = file_bytes.ok_or_else(|| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "FILE_FIELD_MISSING",
            "Не найден обязательный multipart field 'file'.",
            false,
        )
    })?;

    let safe_filename = validate_upload(&filename, &bytes, &state.security_limits)?;

    let job_id = format!("job_{}", Uuid::new_v4().simple());
    let document_id = format!("doc_{}", Uuid::new_v4().simple());

    let input_path = state
        .document_store
        .save_upload(&document_id, &safe_filename, &bytes)
        .await
        .map_err(|_| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "UPLOAD_STORE_FAILED",
                "Не удалось сохранить загруженный файл.",
                true,
            )
        })?;

    let job = Job::new(job_id.clone(), document_id.clone(), input_path, options);
    state.registry.create(job.clone()).await.map_err(|_| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "JOB_CREATE_FAILED",
            "Не удалось создать задачу обработки.",
            true,
        )
    })?;

    state.queue.enqueue(job).await.map_err(|_| {
        ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "QUEUE_FULL",
            "Очередь обработки заполнена.",
            true,
        )
    })?;

    state.metrics.inc("documents_submitted_total").await;
    state.metrics.inc("jobs_queued").await;

    Ok(Json(UploadAcceptedResponse {
        job_id,
        document_id,
        status: JobStatus::Queued,
        message: "Документ принят в обработку.".to_string(),
    }))
}

pub async fn get_job_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> ApiResult<Json<JobStatusResponse>> {
    let Some(job) = state.registry.get(&job_id).await.map_err(|_| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "JOB_LOOKUP_FAILED",
            "Не удалось получить статус задачи.",
            true,
        )
    })? else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "JOB_NOT_FOUND",
            "Задача не найдена.",
            false,
        ));
    };

    Ok(Json(JobStatusResponse {
        job_id: job.job_id,
        document_id: job.document_id,
        status: job.status,
        progress: job.progress,
        created_at: job.created_at.to_rfc3339(),
        updated_at: job.updated_at.to_rfc3339(),
        error: job.error,
    }))
}

pub async fn get_document_model(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
) -> ApiResult<Response> {
    let output_root = PathBuf::from(state.profile.storage.output_dir.clone()).join(&document_id);
    let model_path = output_root.join("model.json");
    if !model_path.exists() {
        let maybe_job = state.registry.by_document_id(&document_id).await.ok().flatten();
        if let Some(job) = maybe_job {
            if matches!(job.status, JobStatus::Queued | JobStatus::Processing) {
                return Err(ApiError::new(
                    StatusCode::CONFLICT,
                    "DOCUMENT_NOT_READY",
                    "Документ ещё обрабатывается.",
                    true,
                ));
            }
        }

        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "DOCUMENT_NOT_FOUND",
            "Документ не найден.",
            false,
        ));
    }

    serve_file(model_path, "application/json").await
}

pub async fn get_document_markdown(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
) -> ApiResult<Response> {
    let output_root = PathBuf::from(state.profile.storage.output_dir.clone()).join(&document_id);
    let path = output_root.join("markdown.md");
    if !path.exists() {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "DOCUMENT_OUTPUT_NOT_FOUND",
            "Файл markdown для документа не найден.",
            false,
        ));
    }
    serve_file(path, "text/markdown; charset=utf-8").await
}

pub async fn get_document_text(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
) -> ApiResult<Response> {
    let output_root = PathBuf::from(state.profile.storage.output_dir.clone()).join(&document_id);
    let path = output_root.join("plain_text.txt");
    if !path.exists() {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "DOCUMENT_OUTPUT_NOT_FOUND",
            "Файл plain text для документа не найден.",
            false,
        ));
    }
    serve_file(path, "text/plain; charset=utf-8").await
}

pub async fn get_document_asset(
    State(state): State<AppState>,
    Path((document_id, asset_id)): Path<(String, String)>,
) -> ApiResult<Response> {
    let output_root = PathBuf::from(state.profile.storage.output_dir.clone()).join(&document_id);
    let model_path = output_root.join("model.json");
    if !model_path.exists() {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "DOCUMENT_NOT_FOUND",
            "Документ не найден.",
            false,
        ));
    }

    let model_bytes = tokio::fs::read(&model_path).await.map_err(|_| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "MODEL_READ_FAILED",
            "Не удалось прочитать model.json.",
            true,
        )
    })?;
    let model_json: serde_json::Value = serde_json::from_slice(&model_bytes).map_err(|_| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "MODEL_PARSE_FAILED",
            "Не удалось разобрать model.json.",
            true,
        )
    })?;

    let assets = model_json
        .get("assets")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let asset = assets.iter().find(|asset| {
        asset
            .get("asset_id")
            .and_then(|v| v.as_str())
            .map(|v| v == asset_id)
            .unwrap_or(false)
    });

    let Some(asset) = asset else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "ASSET_NOT_FOUND",
            "Запрошенный asset не найден.",
            false,
        ));
    };

    let relative = asset
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "ASSET_PATH_INVALID",
                "Некорректный путь asset в model.json.",
                true,
            )
        })?;

    let Some(asset_path) = safe_join(&output_root, relative) else {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ASSET_PATH",
            "Путь asset содержит недопустимые компоненты.",
            false,
        ));
    };

    if !asset_path.exists() {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "ASSET_NOT_FOUND",
            "Файл asset не найден на диске.",
            false,
        ));
    }

    let content_type = asset
        .get("mime_type")
        .and_then(|v| v.as_str())
        .unwrap_or("application/octet-stream");

    serve_file(asset_path, content_type).await
}

async fn serve_file(path: PathBuf, content_type: &str) -> ApiResult<Response> {
    let bytes = tokio::fs::read(path).await.map_err(|_| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "FILE_READ_FAILED",
            "Не удалось прочитать запрошенный файл.",
            true,
        )
    })?;

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        content_type.parse().unwrap_or(axum::http::HeaderValue::from_static("application/octet-stream")),
    );

    Ok((StatusCode::OK, headers, Body::from(bytes)).into_response())
}

pub fn seeded_completed_job(document_id: &str, input_path: PathBuf) -> Job {
    let mut job = Job::new(
        format!("job_{}", Uuid::new_v4().simple()),
        document_id.to_string(),
        input_path,
        ProcessingOptions::default(),
    );
    job.status = JobStatus::Completed;
    job.progress.percent = Some(100.0);
    job.updated_at = Utc::now();
    job
}

pub fn api_error(code: &str, message: &str) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "error": {
            "code": code,
            "message": message,
            "recoverable": false,
            "details": HashMap::<String, serde_json::Value>::new()
        }
    }))
}
