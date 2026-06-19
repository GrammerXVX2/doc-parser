use std::path::PathBuf;

use async_trait::async_trait;

pub type ObjectStoreResult<T> = Result<T, anyhow::Error>;

#[async_trait]
pub trait ObjectStore: Send + Sync {
    async fn put_bytes(&self, key: &str, bytes: &[u8], content_type: &str) -> ObjectStoreResult<()>;
    async fn get_bytes(&self, key: &str) -> ObjectStoreResult<Option<Vec<u8>>>;
}

#[derive(Debug, Clone)]
pub struct LocalObjectStore {
    pub root: PathBuf,
}

impl LocalObjectStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn full_path(&self, key: &str) -> PathBuf {
        self.root.join(key)
    }
}

#[async_trait]
impl ObjectStore for LocalObjectStore {
    async fn put_bytes(&self, key: &str, bytes: &[u8], _content_type: &str) -> ObjectStoreResult<()> {
        let path = self.full_path(key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, bytes).await?;
        Ok(())
    }

    async fn get_bytes(&self, key: &str) -> ObjectStoreResult<Option<Vec<u8>>> {
        let path = self.full_path(key);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(tokio::fs::read(path).await?))
    }
}
