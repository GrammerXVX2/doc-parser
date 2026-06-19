use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use anyhow::anyhow;

use crate::ml::{ExecutionProviderKind, OnnxSession};

#[derive(Debug, Default)]
pub struct SessionRegistry {
    sessions: RwLock<HashMap<String, Arc<OnnxSession>>>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_load(
        &self,
        model_path: &Path,
        provider: ExecutionProviderKind,
    ) -> anyhow::Result<Arc<OnnxSession>> {
        let key = format!("{}::{}", model_path.display(), provider.as_str());

        if let Some(existing) = self
            .sessions
            .read()
            .map_err(|_| anyhow!("MODEL_LOAD_FAILED: session registry read lock poisoned"))?
            .get(&key)
            .cloned()
        {
            return Ok(existing);
        }

        let session = Arc::new(OnnxSession::new(model_path, provider)?);
        let mut guard = self
            .sessions
            .write()
            .map_err(|_| anyhow!("MODEL_LOAD_FAILED: session registry write lock poisoned"))?;
        let entry = guard.entry(key).or_insert_with(|| session.clone());
        Ok(entry.clone())
    }

    pub fn len(&self) -> usize {
        self.sessions.read().map(|guard| guard.len()).unwrap_or(0)
    }
}
