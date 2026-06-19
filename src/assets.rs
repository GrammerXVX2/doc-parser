use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::model::Asset;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    PageRender,
    EmbeddedImage,
    OcrCrop,
    TableHtml,
    TableCsv,
    FormulaImage,
    Debug,
}

impl AssetType {
    pub fn as_str(self) -> &'static str {
        match self {
            AssetType::PageRender => "page_render",
            AssetType::EmbeddedImage => "embedded_image",
            AssetType::OcrCrop => "ocr_crop",
            AssetType::TableHtml => "table_html",
            AssetType::TableCsv => "table_csv",
            AssetType::FormulaImage => "formula_image",
            AssetType::Debug => "debug",
        }
    }

    pub fn folder(self) -> &'static str {
        match self {
            AssetType::PageRender => "renders",
            AssetType::EmbeddedImage => "images",
            AssetType::OcrCrop => "crops",
            AssetType::TableHtml | AssetType::TableCsv => "tables",
            AssetType::FormulaImage => "formulas",
            AssetType::Debug => "debug",
        }
    }
}

pub trait AssetStore {
    fn write_asset(
        &self,
        document_id: &str,
        asset_type: AssetType,
        suggested_name: &str,
        bytes: &[u8],
        mime_type: &str,
    ) -> anyhow::Result<Asset>;
}

#[derive(Debug, Clone)]
pub struct LocalAssetStore {
    pub root_output_dir: PathBuf,
}

impl LocalAssetStore {
    pub fn new(root_output_dir: impl Into<PathBuf>) -> Self {
        Self {
            root_output_dir: root_output_dir.into(),
        }
    }

    pub fn ensure_document_layout(&self, document_id: &str) -> anyhow::Result<PathBuf> {
        let doc_dir = self.root_output_dir.join(document_id);
        let assets_dir = doc_dir.join("assets");

        for folder in ["renders", "images", "tables", "crops", "formulas", "debug"] {
            fs::create_dir_all(assets_dir.join(folder)).with_context(|| {
                format!(
                    "failed to create asset folder '{}' for document '{}': {}",
                    folder,
                    document_id,
                    assets_dir.display()
                )
            })?;
        }

        Ok(doc_dir)
    }

    fn sanitize_name(name: &str) -> String {
        let mut out = String::with_capacity(name.len());
        for ch in name.chars() {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                out.push(ch);
            } else {
                out.push('_');
            }
        }
        if out.is_empty() {
            "asset.bin".to_string()
        } else {
            out
        }
    }

    fn extension_for_mime(mime_type: &str) -> &'static str {
        if mime_type.contains("png") {
            "png"
        } else if mime_type.contains("jpeg") || mime_type.contains("jpg") {
            "jpg"
        } else if mime_type.contains("webp") {
            "webp"
        } else if mime_type.contains("bmp") {
            "bmp"
        } else if mime_type.contains("tiff") {
            "tiff"
        } else if mime_type.contains("html") {
            "html"
        } else if mime_type.contains("csv") {
            "csv"
        } else if mime_type.contains("json") {
            "json"
        } else {
            "bin"
        }
    }
}

impl AssetStore for LocalAssetStore {
    fn write_asset(
        &self,
        document_id: &str,
        asset_type: AssetType,
        suggested_name: &str,
        bytes: &[u8],
        mime_type: &str,
    ) -> anyhow::Result<Asset> {
        let doc_dir = self.ensure_document_layout(document_id)?;
        let asset_id = format!("asset_{}", Uuid::new_v4().simple());
        let ext = Path::new(suggested_name)
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| Self::extension_for_mime(mime_type).to_string());

        let sanitized = Self::sanitize_name(suggested_name);
        let file_name = if sanitized.ends_with(&format!(".{ext}")) {
            format!("{}_{}", asset_id, sanitized)
        } else {
            format!("{}_{}.{}", asset_id, sanitized, ext)
        };

        let relative_path = format!("assets/{}/{}", asset_type.folder(), file_name);
        let absolute_path = doc_dir.join(&relative_path);

        fs::write(&absolute_path, bytes)
            .with_context(|| format!("failed to write asset file: {}", absolute_path.display()))?;

        let sha256 = {
            let mut hasher = Sha256::new();
            hasher.update(bytes);
            format!("{:x}", hasher.finalize())
        };

        Ok(Asset {
            asset_id,
            asset_type: asset_type.as_str().to_string(),
            path: relative_path,
            mime_type: mime_type.to_string(),
            page_number: None,
            width: None,
            height: None,
            dpi: None,
            sha256: Some(sha256),
            provenance: json!({
                "source": "pipeline_asset_store",
                "tool": "local_asset_store",
                "stage": "asset_write",
            }),
            extra: Default::default(),
        })
    }
}
