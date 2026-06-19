use std::sync::Arc;

use document_parser::assets::LocalAssetStore;
use document_parser::ocr::{OcrBackendFactory, OcrBackendKind, OcrConfig};

fn temp_output_dir() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("doc_parser_stage3_{}", uuid::Uuid::new_v4()))
}

#[test]
fn onnx_backend_falls_back_to_mock_when_configured() {
    let mut cfg = OcrConfig::default();
    cfg.backend = OcrBackendKind::Onnx;
    cfg.fallback_to_mock = true;

    let store: Arc<dyn document_parser::assets::AssetStore + Send + Sync> =
        Arc::new(LocalAssetStore::new(temp_output_dir()));

    let (_pipeline, warnings) = OcrBackendFactory::create(&cfg, store).expect("factory should succeed");
    assert!(warnings.iter().any(|w| w.code == "OCR_BACKEND_FALLBACK_TO_MOCK"));
}

#[test]
fn onnx_backend_returns_error_without_fallback() {
    let mut cfg = OcrConfig::default();
    cfg.backend = OcrBackendKind::Onnx;
    cfg.fallback_to_mock = false;

    let store: Arc<dyn document_parser::assets::AssetStore + Send + Sync> =
        Arc::new(LocalAssetStore::new(temp_output_dir()));

    let err = match OcrBackendFactory::create(&cfg, store) {
        Ok(_) => panic!("factory should fail"),
        Err(err) => err,
    };
    let msg = err.to_string();
    assert!(msg.contains("OCR_ONNX_FEATURE_DISABLED") || msg.contains("OCR_MODEL_NOT_FOUND"));
}
