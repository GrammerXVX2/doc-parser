use std::sync::Arc;

use serde_json::json;

use crate::config::PipelineConfig;
use crate::ml::ExecutionProviderKind;
use crate::ocr::detection::OnnxTextDetector;
use crate::ocr::recognition::OnnxTextRecognizer;
use crate::ocr::traits::{TextDetector, TextRecognizer};
use crate::ocr::types::{OcrBackendKind, OcrConfig};
use crate::performance::warmup::{WarmupReport, warmup_detector, warmup_recognizer};

pub struct ModelRegistry {
    ocr_detector: Option<Arc<dyn TextDetector + Send + Sync>>,
    ocr_recognizer: Option<Arc<dyn TextRecognizer + Send + Sync>>,
    warmup_iterations: usize,
    metadata: serde_json::Value,
}

impl ModelRegistry {
    pub async fn load_from_config(config: &PipelineConfig) -> anyhow::Result<Self> {
        let mut ocr_config = OcrConfig::from_pipeline_ocr_value(&config.pipeline.ocr);

        if let Some(provider) = config.pipeline.ml.get("provider").and_then(|v| v.as_str()) {
            let provider = ExecutionProviderKind::from_str(provider);
            ocr_config.detection.provider = provider;
            ocr_config.recognition.provider = provider;
        }

        let warmup_iterations = config
            .pipeline
            .performance
            .get("warmup")
            .and_then(|v| v.get("iterations"))
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as usize;

        if !ocr_config.enabled || matches!(ocr_config.backend, OcrBackendKind::Disabled) {
            return Ok(Self {
                ocr_detector: None,
                ocr_recognizer: None,
                warmup_iterations,
                metadata: json!({
                    "status": "disabled",
                    "backend": "disabled",
                }),
            });
        }

        let detector = match ocr_config.backend {
            OcrBackendKind::Onnx => match OnnxTextDetector::new(ocr_config.detection.clone()) {
                Ok(model) => Some(Arc::new(model) as Arc<dyn TextDetector + Send + Sync>),
                Err(error) if ocr_config.fallback_to_mock => {
                    tracing::warn!(
                        code = "MODEL_LOAD_FAILED",
                        "ONNX detector unavailable, fallback active: {}",
                        error
                    );
                    None
                }
                Err(error) => {
                    return Err(anyhow::anyhow!("MODEL_LOAD_FAILED: {}", error));
                }
            },
            OcrBackendKind::Mock | OcrBackendKind::Triton | OcrBackendKind::Disabled => None,
        };

        let recognizer = match ocr_config.backend {
            OcrBackendKind::Onnx => match OnnxTextRecognizer::new(ocr_config.recognition.clone()) {
                Ok(model) => Some(Arc::new(model) as Arc<dyn TextRecognizer + Send + Sync>),
                Err(error) if ocr_config.fallback_to_mock => {
                    tracing::warn!(
                        code = "MODEL_LOAD_FAILED",
                        "ONNX recognizer unavailable, fallback active: {}",
                        error
                    );
                    None
                }
                Err(error) => {
                    return Err(anyhow::anyhow!("MODEL_LOAD_FAILED: {}", error));
                }
            },
            OcrBackendKind::Mock | OcrBackendKind::Triton | OcrBackendKind::Disabled => None,
        };

        Ok(Self {
            ocr_detector: detector,
            ocr_recognizer: recognizer,
            warmup_iterations,
            metadata: json!({
                "status": "loaded",
                "backend": format!("{:?}", ocr_config.backend).to_lowercase(),
                "provider": ocr_config.recognition.provider.as_str(),
            }),
        })
    }

    pub fn get_ocr_detector(&self) -> Option<Arc<dyn TextDetector + Send + Sync>> {
        self.ocr_detector.clone()
    }

    pub fn get_ocr_recognizer(&self) -> Option<Arc<dyn TextRecognizer + Send + Sync>> {
        self.ocr_recognizer.clone()
    }

    pub fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }

    pub async fn warmup(&self) -> anyhow::Result<WarmupReport> {
        let mut duration_ms = 0_u64;
        let mut status = "ok".to_string();

        if let Some(detector) = &self.ocr_detector {
            match warmup_detector(detector.as_ref(), self.warmup_iterations) {
                Ok(report) => {
                    duration_ms = duration_ms.saturating_add(report.duration_ms);
                }
                Err(error) => {
                    status = format!("MODEL_WARMUP_FAILED: {}", error);
                }
            }
        }

        if let Some(recognizer) = &self.ocr_recognizer {
            match warmup_recognizer(recognizer.as_ref(), self.warmup_iterations) {
                Ok(report) => {
                    duration_ms = duration_ms.saturating_add(report.duration_ms);
                }
                Err(error) => {
                    status = format!("MODEL_WARMUP_FAILED: {}", error);
                }
            }
        }

        Ok(WarmupReport {
            model_name: "ocr_models".to_string(),
            iterations: self.warmup_iterations,
            duration_ms,
            status,
        })
    }
}
