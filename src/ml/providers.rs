use std::path::PathBuf;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MlProvider {
    Cpu,
    Cuda {
        device_id: usize,
    },
    TensorRt {
        device_id: usize,
        engine_cache_path: PathBuf,
        fp16: bool,
    },
    Triton {
        url: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionProviderKind {
    Cpu,
    Cuda,
    TensorRt,
    Triton,
}

impl ExecutionProviderKind {
    pub fn from_str(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "cuda" => Self::Cuda,
            "tensorrt" | "tensor_rt" | "trt" => Self::TensorRt,
            "triton" => Self::Triton,
            _ => Self::Cpu,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Cuda => "cuda",
            Self::TensorRt => "tensorrt",
            Self::Triton => "triton",
        }
    }

    pub fn is_available(&self) -> bool {
        // Stage 9 MVP keeps CPU as always-available default. Other providers are
        // represented in config/runtime, and return structured availability errors
        // until provider-specific runtime integration is enabled.
        matches!(self, Self::Cpu)
    }

    pub fn unavailable_code(&self) -> &'static str {
        match self {
            Self::Cpu => "ML_PROVIDER_UNAVAILABLE",
            Self::Cuda => "CUDA_PROVIDER_UNAVAILABLE",
            Self::TensorRt => "TENSORRT_PROVIDER_UNAVAILABLE",
            Self::Triton => "TRITON_UNAVAILABLE",
        }
    }

    pub fn unavailable_message_ru(&self) -> &'static str {
        match self {
            Self::Cpu => "ML-провайдер недоступен.",
            Self::Cuda => {
                "CUDA-провайдер недоступен. Будет использован CPU или настроенный fallback."
            }
            Self::TensorRt => {
                "TensorRT-провайдер недоступен. Будет использован CPU или настроенный fallback."
            }
            Self::Triton => "Triton backend недоступен. Проверьте конфигурацию и доступность сервера.",
        }
    }

    pub fn ensure_available(&self) -> anyhow::Result<()> {
        if self.is_available() {
            return Ok(());
        }

        Err(anyhow!(
            "{}: {}",
            self.unavailable_code(),
            self.unavailable_message_ru()
        ))
    }

    pub fn to_ml_provider(
        &self,
        device_id: usize,
        trt_engine_cache_path: PathBuf,
        fp16: bool,
        triton_url: Option<String>,
    ) -> MlProvider {
        match self {
            Self::Cpu => MlProvider::Cpu,
            Self::Cuda => MlProvider::Cuda { device_id },
            Self::TensorRt => MlProvider::TensorRt {
                device_id,
                engine_cache_path: trt_engine_cache_path,
                fp16,
            },
            Self::Triton => MlProvider::Triton {
                url: triton_url.unwrap_or_else(|| "http://127.0.0.1:8000".to_string()),
            },
        }
    }
}
