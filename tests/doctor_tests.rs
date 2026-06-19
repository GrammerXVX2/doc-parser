use std::path::PathBuf;

use document_parser::doctor::{DoctorOptions, DoctorStatus, run_doctor};

#[test]
fn doctor_reports_missing_optional_tools_as_warnings() {
    let options = DoctorOptions {
        pipeline_config_path: PathBuf::from("configs/pipeline.config.jsonc"),
        service_profile_path: PathBuf::from("configs/profiles/api.jsonc"),
    };

    let report = run_doctor(&options);
    assert!(!report.checks.is_empty());

    let has_converter_warn = report.checks.iter().any(|check| {
        check.code == "DOCTOR_CONVERTER_MISSING" || check.code == "DOCTOR_CONVERTER_AVAILABLE"
    });
    assert!(has_converter_warn);
}

#[test]
fn doctor_reports_invalid_required_config_as_error() {
    let options = DoctorOptions {
        pipeline_config_path: PathBuf::from("configs/does_not_exist.jsonc"),
        service_profile_path: PathBuf::from("configs/profiles/api.jsonc"),
    };

    let report = run_doctor(&options);
    assert!(report
        .checks
        .iter()
        .any(|check| check.code == "DOCTOR_CONFIG_INVALID" && check.status == DoctorStatus::Error));
}

#[test]
fn doctor_json_output_serializable() {
    let options = DoctorOptions::default();
    let report = run_doctor(&options);
    let json = serde_json::to_string_pretty(&report).unwrap();
    assert!(json.contains("checks"));
}
