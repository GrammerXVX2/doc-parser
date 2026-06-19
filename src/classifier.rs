use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DetectedFormat {
    Pdf,
    Docx,
    Doc,
    Html,
    Md,
    Rtf,
    Image,
    Pptx,
    Txt,
    Xlsx,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileClassification {
    pub input_path: PathBuf,
    pub extension: String,
    pub mime_by_extension: Option<String>,
    pub mime_by_magic: Option<String>,
    pub size_bytes: u64,
    pub sha256: String,
    pub likely_format: DetectedFormat,
    pub is_encrypted_or_protected: bool,
}

pub fn classify_file(input_path: &Path) -> anyhow::Result<FileClassification> {
    let metadata = fs::metadata(input_path)
        .with_context(|| format!("failed to stat input file: {}", input_path.display()))?;
    let bytes = fs::read(input_path)
        .with_context(|| format!("failed to read input file: {}", input_path.display()))?;

    let extension = input_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mime_by_extension = mime_guess::from_path(input_path)
        .first_raw()
        .map(str::to_string);

    let mime_by_magic = infer::get(&bytes).map(|k| k.mime_type().to_string());

    let sha256 = {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        format!("{:x}", hasher.finalize())
    };

    let likely_format = detect_format(&extension, mime_by_extension.as_deref(), mime_by_magic.as_deref());
    let is_encrypted_or_protected = detect_protection(&likely_format, &bytes);

    Ok(FileClassification {
        input_path: input_path.to_path_buf(),
        extension,
        mime_by_extension,
        mime_by_magic,
        size_bytes: metadata.len(),
        sha256,
        likely_format,
        is_encrypted_or_protected,
    })
}

fn detect_format(extension: &str, mime_ext: Option<&str>, mime_magic: Option<&str>) -> DetectedFormat {
    let mime = mime_magic.or(mime_ext).unwrap_or_default().to_ascii_lowercase();

    if extension == "pdf" || mime == "application/pdf" {
        return DetectedFormat::Pdf;
    }
    if extension == "docx"
        || mime == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    {
        return DetectedFormat::Docx;
    }
    if extension == "doc" || mime == "application/msword" {
        return DetectedFormat::Doc;
    }
    if extension == "html" || extension == "htm" || mime == "text/html" {
        return DetectedFormat::Html;
    }
    if extension == "md" || extension == "markdown" || mime == "text/markdown" {
        return DetectedFormat::Md;
    }
    if extension == "rtf" || mime == "application/rtf" || mime == "text/rtf" {
        return DetectedFormat::Rtf;
    }
    if extension == "pptx"
        || mime == "application/vnd.openxmlformats-officedocument.presentationml.presentation"
    {
        return DetectedFormat::Pptx;
    }
    if extension == "xlsx"
        || mime == "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    {
        return DetectedFormat::Xlsx;
    }
    if matches!(
        extension,
        "jpg" | "jpeg" | "png" | "tif" | "tiff" | "webp" | "bmp"
    ) || mime.starts_with("image/")
    {
        return DetectedFormat::Image;
    }
    if extension == "txt" || extension == "log" || mime == "text/plain" {
        return DetectedFormat::Txt;
    }

    DetectedFormat::Unknown
}

fn detect_protection(format: &DetectedFormat, bytes: &[u8]) -> bool {
    if matches!(format, DetectedFormat::Pdf) {
        // Cheap heuristic for encrypted PDF files.
        let text = String::from_utf8_lossy(bytes);
        return text.contains("/Encrypt");
    }
    false
}
