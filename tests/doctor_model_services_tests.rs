use std::path::PathBuf;

use document_parser::doctor::{DoctorOptions, DoctorStatus, run_doctor};

#[test]
fn doctor_reports_service_unavailable_as_warn_when_not_required() {
    let options = DoctorOptions {
        pipeline_config_path: PathBuf::from("configs/pipeline.config.jsonc"),
        service_profile_path: PathBuf::from("configs/profiles/api.jsonc"),
        model_stack_config_path: PathBuf::from("configs/model_stack.config.jsonc"),
    };

    let report = run_doctor(&options);

    let has_warn = report.checks.iter().any(|c| {
        (c.code == "PADDLEOCR_SERVICE_UNAVAILABLE"
            || c.code == "SURYA_SERVICE_UNAVAILABLE"
            || c.code == "DOCLING_SERVICE_UNAVAILABLE")
            && c.status == DoctorStatus::Warn
    });

    assert!(has_warn || report.checks.iter().any(|c| c.code == "DOCTOR_MODEL_SERVICE_READY"));
}

#[test]
fn doctor_reports_error_when_required_service_missing() {
    let mut value: serde_json::Value = document_parser::config::load_jsonc_file(PathBuf::from("configs/model_stack.config.jsonc").as_path()).unwrap();

    if let Some(backends) = value
        .get_mut("model_stack")
        .and_then(|v| v.get_mut("backends"))
        .and_then(|v| v.as_object_mut())
    {
        if let Some(paddle) = backends.get_mut("paddleocr_ppocrv6_medium") {
            paddle["required"] = serde_json::json!(true);
            paddle["url"] = serde_json::json!("http://127.0.0.1:39999");
        }
    }

    let tmp = std::env::temp_dir().join(format!("model_stack_required_{}.jsonc", uuid::Uuid::new_v4()));
    std::fs::write(&tmp, serde_json::to_string_pretty(&value).unwrap()).unwrap();

    let options = DoctorOptions {
        pipeline_config_path: PathBuf::from("configs/pipeline.config.jsonc"),
        service_profile_path: PathBuf::from("configs/profiles/api.jsonc"),
        model_stack_config_path: tmp,
    };

    let report = run_doctor(&options);

    assert!(report.checks.iter().any(|c| {
        c.code == "PADDLEOCR_SERVICE_UNAVAILABLE" && c.status == DoctorStatus::Error
    }));
}
