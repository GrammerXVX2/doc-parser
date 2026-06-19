use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::config::profiles::ServiceProfile;
use crate::config::{PipelineConfig, load_pipeline_config};
use crate::ocr::{OcrBackendKind, OcrConfig};
use crate::utils::command_exists::resolve_command_path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DoctorStatus {
    Ok,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorCheck {
    pub code: String,
    pub status: DoctorStatus,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub generated_at: String,
    pub checks: Vec<DoctorCheck>,
}

impl DoctorReport {
    pub fn overall_status(&self) -> DoctorStatus {
        if self.checks.iter().any(|c| c.status == DoctorStatus::Error) {
            DoctorStatus::Error
        } else if self.checks.iter().any(|c| c.status == DoctorStatus::Warn) {
            DoctorStatus::Warn
        } else {
            DoctorStatus::Ok
        }
    }
}

#[derive(Debug, Clone)]
pub struct DoctorOptions {
    pub pipeline_config_path: PathBuf,
    pub service_profile_path: PathBuf,
}

impl Default for DoctorOptions {
    fn default() -> Self {
        Self {
            pipeline_config_path: PathBuf::from("configs/pipeline.config.jsonc"),
            service_profile_path: PathBuf::from("configs/profiles/api.jsonc"),
        }
    }
}

pub fn run_doctor(options: &DoctorOptions) -> DoctorReport {
    let mut checks = Vec::new();

    let pipeline = match load_pipeline_config(&options.pipeline_config_path) {
        Ok(cfg) => {
            checks.push(DoctorCheck {
                code: "DOCTOR_CONFIG_LOADED".to_string(),
                status: DoctorStatus::Ok,
                message: "Конфигурация pipeline загружена.".to_string(),
            });
            Some(cfg)
        }
        Err(error) => {
            checks.push(DoctorCheck {
                code: "DOCTOR_CONFIG_INVALID".to_string(),
                status: DoctorStatus::Error,
                message: format!("Ошибка загрузки pipeline-конфига: {}", error),
            });
            None
        }
    };

    let profile = match ServiceProfile::from_path(&options.service_profile_path) {
        Ok(profile) => {
            checks.push(DoctorCheck {
                code: "DOCTOR_SERVICE_PROFILE_LOADED".to_string(),
                status: DoctorStatus::Ok,
                message: "Сервисный профиль загружен.".to_string(),
            });
            Some(profile)
        }
        Err(error) => {
            checks.push(DoctorCheck {
                code: "DOCTOR_CONFIG_INVALID".to_string(),
                status: DoctorStatus::Error,
                message: format!("Ошибка загрузки service profile: {}", error),
            });
            None
        }
    };

    if let Some(profile) = &profile {
        check_storage_writable(Path::new(&profile.storage.output_dir), "output", &mut checks);
        check_storage_writable(Path::new(&profile.storage.input_dir), "input", &mut checks);
        check_security_limits(profile, &mut checks);

        if profile.observability.prometheus_enabled && profile.observability.prometheus_path.trim().is_empty() {
            checks.push(DoctorCheck {
                code: "DOCTOR_CONFIG_INVALID".to_string(),
                status: DoctorStatus::Error,
                message: "Prometheus включен, но путь метрик пустой.".to_string(),
            });
        } else if profile.observability.prometheus_enabled {
            checks.push(DoctorCheck {
                code: "DOCTOR_PROMETHEUS_ENABLED".to_string(),
                status: DoctorStatus::Ok,
                message: format!(
                    "Prometheus endpoint включен: {}",
                    profile.observability.prometheus_path
                ),
            });
        } else {
            checks.push(DoctorCheck {
                code: "DOCTOR_PROMETHEUS_DISABLED".to_string(),
                status: DoctorStatus::Warn,
                message: "Prometheus endpoint отключен в сервисном профиле.".to_string(),
            });
        }
    }

    check_converter("soffice", "LibreOffice", &mut checks);
    check_converter("pandoc", "Pandoc", &mut checks);

    if let Some(pipeline) = &pipeline {
        check_ocr_models(pipeline, &mut checks);
        check_triton(pipeline, &mut checks);
    }

    DoctorReport {
        generated_at: Utc::now().to_rfc3339(),
        checks,
    }
}

pub fn render_doctor_report_text(report: &DoctorReport) -> String {
    let mut out = String::new();
    out.push_str("Проверка окружения document-parser\n\n");

    for check in &report.checks {
        let level = match check.status {
            DoctorStatus::Ok => "OK",
            DoctorStatus::Warn => "WARN",
            DoctorStatus::Error => "ERROR",
        };
        out.push_str(&format!("{}: {}\n", level, check.message));
    }

    out
}

fn check_storage_writable(path: &Path, kind: &str, checks: &mut Vec<DoctorCheck>) {
    let result = std::fs::create_dir_all(path)
        .and_then(|_| {
            let probe = path.join(format!(".doctor_probe_{}", uuid::Uuid::new_v4()));
            let mut file = std::fs::File::create(&probe)?;
            file.write_all(b"ok")?;
            std::fs::remove_file(probe)
        });

    match result {
        Ok(_) => checks.push(DoctorCheck {
            code: "DOCTOR_STORAGE_AVAILABLE".to_string(),
            status: DoctorStatus::Ok,
            message: format!("Каталог {} доступен для записи: {}", kind, path.display()),
        }),
        Err(error) => checks.push(DoctorCheck {
            code: "DOCTOR_STORAGE_UNAVAILABLE".to_string(),
            status: DoctorStatus::Error,
            message: format!(
                "Каталог {} недоступен для записи ({}): {}",
                kind,
                path.display(),
                error
            ),
        }),
    }
}

fn check_converter(binary: &str, title: &str, checks: &mut Vec<DoctorCheck>) {
    if resolve_command_path(binary).is_some() {
        checks.push(DoctorCheck {
            code: "DOCTOR_CONVERTER_AVAILABLE".to_string(),
            status: DoctorStatus::Ok,
            message: format!("{} доступен в PATH.", title),
        });
    } else {
        checks.push(DoctorCheck {
            code: "DOCTOR_CONVERTER_MISSING".to_string(),
            status: DoctorStatus::Warn,
            message: format!("{} не найден в PATH.", title),
        });
    }
}

fn check_ocr_models(pipeline: &PipelineConfig, checks: &mut Vec<DoctorCheck>) {
    let ocr = OcrConfig::from_pipeline_ocr_value(&pipeline.pipeline.ocr);
    if !ocr.enabled || !matches!(ocr.backend, OcrBackendKind::Onnx) {
        checks.push(DoctorCheck {
            code: "DOCTOR_MODEL_CHECK_SKIPPED".to_string(),
            status: DoctorStatus::Warn,
            message: "Проверка ONNX-моделей пропущена: OCR ONNX не активирован.".to_string(),
        });
        return;
    }

    for path in [&ocr.detection.model_path, &ocr.recognition.model_path, &ocr.recognition.charset_path] {
        if Path::new(path).exists() {
            checks.push(DoctorCheck {
                code: "DOCTOR_MODEL_AVAILABLE".to_string(),
                status: DoctorStatus::Ok,
                message: format!("Найден файл модели/ресурса: {}", path),
            });
        } else {
            checks.push(DoctorCheck {
                code: "DOCTOR_MODEL_MISSING".to_string(),
                status: DoctorStatus::Warn,
                message: format!("Файл модели/ресурса не найден: {}", path),
            });
        }
    }
}

fn check_triton(pipeline: &PipelineConfig, checks: &mut Vec<DoctorCheck>) {
    let provider = pipeline
        .pipeline
        .ml
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("cpu");

    let triton_enabled = pipeline
        .pipeline
        .ml
        .get("triton")
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !triton_enabled && !provider.eq_ignore_ascii_case("triton") {
        checks.push(DoctorCheck {
            code: "DOCTOR_TRITON_DISABLED".to_string(),
            status: DoctorStatus::Warn,
            message: "Triton backend отключен в конфигурации.".to_string(),
        });
        return;
    }

    let url = pipeline
        .pipeline
        .ml
        .get("triton")
        .and_then(|v| v.get("url"))
        .and_then(|v| v.as_str())
        .unwrap_or("http://127.0.0.1:8000")
        .to_string();

    match crate::ml::TritonClient::new(url.clone()).and_then(|client| client.ensure_ready()) {
        Ok(_) => checks.push(DoctorCheck {
            code: "DOCTOR_TRITON_READY".to_string(),
            status: DoctorStatus::Ok,
            message: format!("Triton доступен: {}", url),
        }),
        Err(error) => checks.push(DoctorCheck {
            code: "DOCTOR_TRITON_UNAVAILABLE".to_string(),
            status: DoctorStatus::Warn,
            message: format!("Triton недоступен ({}): {}", url, error),
        }),
    }
}

fn check_security_limits(profile: &ServiceProfile, checks: &mut Vec<DoctorCheck>) {
    let sec = &profile.security;
    let sane = sec.max_file_size_mb > 0
        && sec.max_pages_per_document > 0
        && sec.max_processing_time_sec > 0
        && sec.max_archive_entries > 0
        && sec.max_archive_total_uncompressed_mb > 0
        && sec.max_image_width_px > 0
        && sec.max_image_height_px > 0;

    if sane {
        checks.push(DoctorCheck {
            code: "DOCTOR_SECURITY_LIMITS_SANE".to_string(),
            status: DoctorStatus::Ok,
            message: "Базовые security-лимиты выглядят корректными.".to_string(),
        });
    } else {
        checks.push(DoctorCheck {
            code: "DOCTOR_CONFIG_INVALID".to_string(),
            status: DoctorStatus::Error,
            message: "Обнаружены некорректные security-лимиты (нулевые/отрицательные значения)."
                .to_string(),
        });
    }
}
