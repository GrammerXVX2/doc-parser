use axum::http::StatusCode;
use std::io::Cursor;

use crate::api::errors::ApiError;
use crate::classifier::DetectedFormat;
use crate::security::limits::SecurityLimits;
use crate::security::safe_paths::sanitize_filename;

pub fn validate_upload(filename: &str, bytes: &[u8], limits: &SecurityLimits) -> Result<String, ApiError> {
    if bytes.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "EMPTY_FILE",
            "Файл пустой и не может быть обработан.",
            false,
        ));
    }

    let max_bytes = limits.max_file_size_mb * 1024 * 1024;
    if bytes.len() as u64 > max_bytes {
        return Err(ApiError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "DOCUMENT_TOO_LARGE",
            "Размер документа превышает допустимый лимит.",
            false,
        ));
    }

    let safe_name = sanitize_filename(filename).ok_or_else(|| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILENAME",
            "Имя файла содержит недопустимые символы.",
            false,
        )
    })?;

    let ext = std::path::Path::new(&safe_name)
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let format = match ext.as_str() {
        "pdf" => DetectedFormat::Pdf,
        "docx" => DetectedFormat::Docx,
        "doc" => DetectedFormat::Doc,
        "html" | "htm" => DetectedFormat::Html,
        "md" | "markdown" => DetectedFormat::Md,
        "rtf" => DetectedFormat::Rtf,
        "png" | "jpg" | "jpeg" | "bmp" | "tif" | "tiff" | "webp" => DetectedFormat::Image,
        "pptx" => DetectedFormat::Pptx,
        "txt" | "log" => DetectedFormat::Txt,
        "xlsx" => DetectedFormat::Xlsx,
        _ => DetectedFormat::Unknown,
    };

    if matches!(format, DetectedFormat::Unknown) {
        return Err(ApiError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "UNSUPPORTED_FILE_TYPE",
            "Тип файла не поддерживается текущим пайплайном.",
            false,
        ));
    }

    if matches!(format, DetectedFormat::Docx | DetectedFormat::Xlsx | DetectedFormat::Pptx) {
        validate_archive_limits(bytes, limits)?;
    }

    if matches!(format, DetectedFormat::Image) {
        if let Ok((w, h)) = crate::utils::image_io::image_dimensions(bytes) {
            if w > limits.max_image_width_px || h > limits.max_image_height_px {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "IMAGE_DIMENSIONS_TOO_LARGE",
                    "Размеры изображения превышают допустимый лимит.",
                    false,
                ));
            }
        }
    }

    Ok(safe_name)
}

fn validate_archive_limits(bytes: &[u8], limits: &SecurityLimits) -> Result<(), ApiError> {
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "UNSUPPORTED_FILE_TYPE",
            "Архив документа поврежден или имеет некорректный формат.",
            false,
        )
    })?;

    if archive.len() > limits.max_archive_entries {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "ARCHIVE_TOO_MANY_ENTRIES",
            "Архив документа содержит слишком много файлов.",
            false,
        ));
    }

    let mut total_uncompressed = 0_u64;
    let max_uncompressed = limits.max_archive_total_uncompressed_mb * 1024 * 1024;

    for idx in 0..archive.len() {
        let entry = archive.by_index(idx).map_err(|_| {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "UNSUPPORTED_FILE_TYPE",
                "Не удалось прочитать содержимое архива документа.",
                false,
            )
        })?;
        total_uncompressed = total_uncompressed.saturating_add(entry.size());
        if total_uncompressed > max_uncompressed {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "ARCHIVE_UNCOMPRESSED_SIZE_TOO_LARGE",
                "Общий размер распакованных данных архива превышает лимит.",
                false,
            ));
        }
    }

    Ok(())
}
