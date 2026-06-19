use serde_json::json;

use document_parser::config::PipelineConfig;
use document_parser::performance::ModelRegistry;

fn config_from(value: serde_json::Value) -> PipelineConfig {
    serde_json::from_value(value).expect("valid pipeline config")
}

#[tokio::test]
async fn registry_loads_mock_configuration() {
    let config = config_from(json!({
        "pipeline": {
            "version": "1.0.0",
            "mode": "bulk",
            "ocr": {
                "enabled": true,
                "backend": "mock",
                "fallback_to_mock": true,
                "languages": ["ru", "en"],
                "language_hint": "ru",
                "locale": "ru"
            },
            "performance": {
                "warmup": {
                    "enabled": true,
                    "iterations": 1
                }
            },
            "ml": {
                "provider": "cpu"
            }
        }
    }));

    let registry = ModelRegistry::load_from_config(&config).await.unwrap();
    assert!(registry.get_ocr_detector().is_none());
    assert!(registry.get_ocr_recognizer().is_none());
}

#[tokio::test]
async fn missing_onnx_model_returns_structured_error() {
    let config = config_from(json!({
        "pipeline": {
            "version": "1.0.0",
            "mode": "bulk",
            "ocr": {
                "enabled": true,
                "backend": "onnx",
                "fallback_to_mock": false,
                "detection": { "model_path": "missing/det.onnx", "provider": "cpu" },
                "recognition": { "model_path": "missing/rec.onnx", "provider": "cpu", "charset_path": "missing/charset.txt" }
            },
            "performance": {
                "warmup": {
                    "enabled": true,
                    "iterations": 1
                }
            },
            "ml": {
                "provider": "cpu"
            }
        }
    }));

    let err = match ModelRegistry::load_from_config(&config).await {
        Ok(_) => panic!("registry should fail"),
        Err(err) => err.to_string(),
    };
    assert!(err.contains("MODEL_LOAD_FAILED"));
}

#[tokio::test]
async fn fallback_to_mock_allows_registry_creation() {
    let config = config_from(json!({
        "pipeline": {
            "version": "1.0.0",
            "mode": "bulk",
            "ocr": {
                "enabled": true,
                "backend": "onnx",
                "fallback_to_mock": true,
                "detection": { "model_path": "missing/det.onnx", "provider": "cpu" },
                "recognition": { "model_path": "missing/rec.onnx", "provider": "cpu", "charset_path": "missing/charset.txt" }
            },
            "performance": {
                "warmup": {
                    "enabled": true,
                    "iterations": 1
                }
            },
            "ml": {
                "provider": "cpu"
            }
        }
    }));

    let registry = ModelRegistry::load_from_config(&config).await.unwrap();
    let report = registry.warmup().await.unwrap();
    assert_eq!(report.iterations, 1);
}
