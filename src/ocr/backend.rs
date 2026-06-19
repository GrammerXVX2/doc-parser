use std::sync::Arc;
use std::time::Duration;

use crate::assets::AssetStore;
use crate::ocr::async_batching::{BatchedTextRecognizer, SyncRecognizerBatchBackend};
use crate::ocr::detection::OnnxTextDetector;
use crate::ocr::mock_ocr::{MockOcrConfig, MockOcrPipeline};
use crate::ocr::onnx_ocr::OnnxOcrPipeline;
use crate::ocr::recognition::OnnxTextRecognizer;
use crate::ocr::traits::{OcrPipeline, TextRecognizer};
use crate::ocr::triton_ocr::{TritonOcrConfig, TritonOcrDetector, TritonOcrRecognizer};
use crate::ocr::types::{OcrBackendKind, OcrConfig};

#[derive(Debug, Clone)]
pub struct OcrBackendWarning {
    pub code: String,
    pub message: String,
}

pub struct OcrBackendFactory;

impl OcrBackendFactory {
    pub fn create(
        config: &OcrConfig,
        asset_store: Arc<dyn AssetStore + Send + Sync>,
    ) -> anyhow::Result<(Box<dyn OcrPipeline + Send + Sync>, Vec<OcrBackendWarning>)> {
        if !config.enabled || matches!(config.backend, OcrBackendKind::Disabled) {
            return Ok((Box::new(DisabledOcrPipeline), vec![]));
        }

        if matches!(config.backend, OcrBackendKind::Mock) {
            return Ok((Box::new(mock_pipeline(config)), vec![]));
        }

        let built = match config.backend {
            OcrBackendKind::Onnx => build_onnx_pipeline(config, asset_store),
            OcrBackendKind::Triton => build_triton_pipeline(config, asset_store),
            OcrBackendKind::Disabled | OcrBackendKind::Mock => {
                unreachable!("mock/disabled are handled above")
            }
        };

        match built {
            Ok(pipeline) => Ok((Box::new(pipeline), vec![])),
            Err(error) if config.fallback_to_mock => {
                let backend = match config.backend {
                    OcrBackendKind::Triton => "Triton",
                    _ => "ONNX",
                };
                Ok((
                    Box::new(mock_pipeline(config)),
                    vec![OcrBackendWarning {
                        code: "OCR_BACKEND_FALLBACK_TO_MOCK".to_string(),
                        message: format!(
                            "{} OCR недоступен; используется mock OCR: {}",
                            backend, error
                        ),
                    }],
                ))
            }
            Err(error) => Err(error),
        }
    }
}

fn mock_pipeline(config: &OcrConfig) -> MockOcrPipeline {
    MockOcrPipeline {
        config: MockOcrConfig {
            fixture_based: config.mock_fixture_based,
            deterministic_fallback: config.mock_deterministic_fallback,
        },
    }
}

fn build_onnx_pipeline(
    config: &OcrConfig,
    asset_store: Arc<dyn AssetStore + Send + Sync>,
) -> anyhow::Result<OnnxOcrPipeline> {
    let detector = OnnxTextDetector::new(config.detection.clone())?;
    let recognizer = build_onnx_recognizer(config)?;

    Ok(OnnxOcrPipeline::new(
        Box::new(detector),
        recognizer,
        asset_store,
        config.clone(),
    ))
}

fn build_onnx_recognizer(config: &OcrConfig) -> anyhow::Result<Box<dyn TextRecognizer + Send + Sync>> {
    if !config.recognition_batching_enabled {
        return Ok(Box::new(OnnxTextRecognizer::new(config.recognition.clone())?));
    }

    let base: Arc<dyn TextRecognizer + Send + Sync> =
        Arc::new(OnnxTextRecognizer::new(config.recognition.clone())?);
    let backend = Arc::new(SyncRecognizerBatchBackend::new(base));
    let recognizer = BatchedTextRecognizer::new(
        backend,
        config.recognition_batching_max_batch_size.max(1),
        Duration::from_millis(config.recognition_batching_max_wait_ms.max(1)),
    );
    Ok(Box::new(recognizer))
}

fn build_triton_pipeline(
    config: &OcrConfig,
    asset_store: Arc<dyn AssetStore + Send + Sync>,
) -> anyhow::Result<OnnxOcrPipeline> {
    let triton_config = TritonOcrConfig {
        enabled: config.triton.enabled,
        url: config.triton.url.clone(),
        grpc_url: config.triton.grpc_url.clone(),
        det_model_name: config.triton.det_model_name.clone(),
        rec_model_name: config.triton.rec_model_name.clone(),
        layout_model_name: config.triton.layout_model_name.clone(),
    };

    let detector = TritonOcrDetector::new(&triton_config)?;
    let recognizer = TritonOcrRecognizer::new(&triton_config)?;

    Ok(OnnxOcrPipeline::new(
        Box::new(detector),
        Box::new(recognizer),
        asset_store,
        config.clone(),
    ))
}

#[derive(Debug, Default)]
struct DisabledOcrPipeline;

impl OcrPipeline for DisabledOcrPipeline {
    fn run_page_ocr(&self, _input: crate::ocr::types::OcrPageInput) -> anyhow::Result<Vec<crate::model::Element>> {
        Ok(vec![])
    }
}
