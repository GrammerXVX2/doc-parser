use std::path::PathBuf;

use document_parser::models::config::{load_model_stack_config, load_model_stack_config_or_default};

#[test]
fn config_loads_and_has_expected_defaults() {
    let path = PathBuf::from("configs/model_stack.config.jsonc");
    let cfg = load_model_stack_config(&path).expect("model stack config");

    assert_eq!(cfg.model_stack.default_language, "ru");
    assert_eq!(cfg.model_stack.locale, "ru");
    assert!(cfg.model_stack.fallback_languages.contains(&"en".to_string()));

    let ppocr = cfg
        .model_stack
        .backends
        .get("paddleocr_ppocrv6_medium")
        .expect("ppocr backend");
    assert_eq!(ppocr.detection_model.as_deref(), Some("PaddlePaddle/PP-OCRv6_medium_det"));
    assert_eq!(ppocr.recognition_model.as_deref(), Some("PaddlePaddle/PP-OCRv6_medium_rec"));

    assert!(cfg.model_stack.backends.contains_key("docling"));
    assert!(cfg.model_stack.backends.contains_key("surya_ocr"));
    assert!(cfg.model_stack.backends.contains_key("paddleocr_vl_1_6"));
    assert!(cfg.model_stack.backends.contains_key("gliner_large_v2_5"));
    assert!(cfg.model_stack.backends.contains_key("gliner_medium_v2_5"));
    assert!(cfg.model_stack.backends.contains_key("gliner_small_v2_5"));
    assert!(cfg.model_stack.backends.contains_key("deepvk_user_bge_m3"));
    assert!(cfg.model_stack.backends.contains_key("baai_bge_m3"));
}

#[test]
fn missing_config_falls_back_to_safe_default() {
    let cfg = load_model_stack_config_or_default(Some(PathBuf::from("configs/missing_model_stack.jsonc").as_path()));
    assert_eq!(cfg.model_stack.default_language, "ru");
    assert_eq!(cfg.model_stack.routing.default_profile, "mixed_enterprise");
}
