use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};

use crate::ml::{ExecutionProviderKind, OnnxTensor};

pub type OnnxInputs = Vec<OnnxTensor>;
pub type OnnxOutputs = Vec<OnnxTensor>;

#[derive(Debug, Clone)]
pub struct OnnxSession {
    pub model_path: PathBuf,
    pub provider: ExecutionProviderKind,
}

impl OnnxSession {
    pub fn new(model_path: &Path, provider: ExecutionProviderKind) -> anyhow::Result<Self> {
        if !cfg!(feature = "onnx") {
            return Err(anyhow!(
                "OCR_ONNX_FEATURE_DISABLED: onnx feature is disabled at compile time"
            ));
        }

        if matches!(provider, ExecutionProviderKind::Triton) {
            return Err(anyhow!(
                "TRITON_UNAVAILABLE: Triton provider requires Triton backend instead of local ONNX session"
            ));
        }

        provider.ensure_available()?;

        if !model_path.exists() {
            return Err(anyhow!(
                "MODEL_LOAD_FAILED: OCR_MODEL_NOT_FOUND: model was not found at {}",
                model_path.display()
            ));
        }

        Ok(Self {
            model_path: model_path.to_path_buf(),
            provider,
        })
    }

    pub fn run(&self, _inputs: OnnxInputs) -> anyhow::Result<OnnxOutputs> {
        #[cfg(feature = "onnx")]
        {
            // Stage 3 MVP architecture: real runtime wiring point. Runtime-specific integration
            // can be introduced behind this method without changing OCR pipeline interfaces.
            Ok(vec![])
        }

        #[cfg(not(feature = "onnx"))]
        {
            Err(anyhow!(
                "OCR_ONNX_FEATURE_DISABLED: onnx feature is disabled at compile time"
            ))
        }
    }

    pub fn load_charset(path: &Path) -> anyhow::Result<Vec<String>> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read charset file: {}", path.display()))?;
        Ok(raw
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect())
    }
}
