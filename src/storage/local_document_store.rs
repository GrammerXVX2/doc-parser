use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::security::safe_paths::sanitize_filename;
use crate::storage::metadata_store::StoreResult;

#[async_trait]
pub trait DocumentStore: Send + Sync {
    async fn save_upload(&self, document_id: &str, filename: &str, bytes: &[u8]) -> StoreResult<PathBuf>;
    async fn get_input_path(&self, document_id: &str) -> StoreResult<PathBuf>;
}

#[derive(Debug, Clone)]
pub struct LocalDocumentStore {
    pub input_root: PathBuf,
}

impl LocalDocumentStore {
    pub fn new(input_root: impl Into<PathBuf>) -> Self {
        Self {
            input_root: input_root.into(),
        }
    }

    fn doc_dir(&self, document_id: &str) -> PathBuf {
        self.input_root.join(document_id)
    }
}

#[async_trait]
impl DocumentStore for LocalDocumentStore {
    async fn save_upload(&self, document_id: &str, filename: &str, bytes: &[u8]) -> StoreResult<PathBuf> {
        let safe = sanitize_filename(filename).ok_or_else(|| anyhow::anyhow!("invalid filename"))?;
        let dir = self.doc_dir(document_id);
        tokio::fs::create_dir_all(&dir).await?;
        let path = dir.join(safe);
        tokio::fs::write(&path, bytes).await?;
        Ok(path)
    }

    async fn get_input_path(&self, document_id: &str) -> StoreResult<PathBuf> {
        let dir = self.doc_dir(document_id);
        if !dir.exists() {
            return Err(anyhow::anyhow!("document input path not found"));
        }
        let mut rd = tokio::fs::read_dir(&dir).await?;
        while let Some(entry) = rd.next_entry().await? {
            if entry.file_type().await?.is_file() {
                return Ok(entry.path());
            }
        }
        Err(anyhow::anyhow!("input file not found for document"))
    }
}

pub fn file_name_from_path(path: &Path) -> String {
    path.file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("document.bin")
        .to_string()
}
