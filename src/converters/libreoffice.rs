use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::Value;

use crate::config::{PipelineConfig, pipeline_converters_value};
use crate::converters::external_process::{ExternalCommand, run_external_command};
use crate::converters::sandbox::{SandboxOptions, prepare_workspace};
use crate::converters::traits::{
    ConversionError, ConversionStageRecord, ConversionTarget, ConvertedDocument, DocumentConverter,
    ExtractionContext, Result,
};
use crate::utils::command_exists::resolve_command_path;
use crate::utils::tempdir::persist_temp_file;

#[derive(Debug, Clone)]
pub struct LibreOfficeConverter {
    pub binary: String,
    pub headless: bool,
    pub sandbox: SandboxOptions,
    pub enabled: bool,
}

impl Default for LibreOfficeConverter {
    fn default() -> Self {
        Self {
            binary: "soffice".to_string(),
            headless: true,
            sandbox: SandboxOptions::default(),
            enabled: true,
        }
    }
}

impl LibreOfficeConverter {
    pub fn from_pipeline_config(config: Option<&PipelineConfig>) -> Self {
        let mut out = Self::default();
        out.sandbox = SandboxOptions::from_pipeline_config(config);

        let Some(config) = config else {
            return out;
        };
        let libre = pipeline_converters_value(config, "libreoffice").unwrap_or(&Value::Null);

        out.enabled = libre.get("enabled").and_then(Value::as_bool).unwrap_or(true);
        out.binary = libre
            .get("binary")
            .and_then(Value::as_str)
            .unwrap_or("soffice")
            .to_string();
        out.headless = libre.get("headless").and_then(Value::as_bool).unwrap_or(true);

        out
    }

    fn resolve_binary(&self) -> Result<PathBuf> {
        resolve_command_path(&self.binary).ok_or_else(|| {
            ConversionError::new(
                "LIBREOFFICE_NOT_AVAILABLE",
                "LibreOffice не найден. Конвертация legacy-документа невозможна.",
            )
        })
    }

    fn supports_extension(ext: &str) -> bool {
        matches!(ext, "doc" | "rtf" | "docx" | "pptx" | "xlsx")
    }

    fn format_for_target(target: ConversionTarget) -> Option<&'static str> {
        match target {
            ConversionTarget::Docx => Some("docx"),
            ConversionTarget::Pdf => Some("pdf"),
            ConversionTarget::Html => Some("html"),
            _ => None,
        }
    }
}

#[async_trait]
impl DocumentConverter for LibreOfficeConverter {
    fn name(&self) -> &'static str {
        "libreoffice"
    }

    fn supports_conversion(&self, input_path: &Path, target: ConversionTarget) -> bool {
        if !self.enabled {
            return false;
        }
        let ext = input_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        Self::supports_extension(&ext) && Self::format_for_target(target).is_some()
    }

    async fn convert(
        &self,
        input_path: &Path,
        target: ConversionTarget,
        context: &mut ExtractionContext,
    ) -> Result<ConvertedDocument> {
        let binary = self.resolve_binary()?;
        let output_format = Self::format_for_target(target).ok_or_else(|| {
            ConversionError::new(
                "LIBREOFFICE_CONVERSION_FAILED",
                format!("LibreOffice не поддерживает цель конвертации '{}'.", target.as_str()),
            )
        })?;

        let workspace = prepare_workspace(input_path, &self.sandbox, "lo_sandbox")?;

        context.push_stage(
            ConversionStageRecord::ok("external_command_start", self.name())
                .with_meta("binary", binary.to_string_lossy())
                .with_meta("target", target.as_str()),
        );

        let mut args = Vec::new();
        if self.headless {
            args.push("--headless".to_string());
        }
        args.push("--convert-to".to_string());
        args.push(output_format.to_string());
        args.push("--outdir".to_string());
        args.push(workspace.output_dir.to_string_lossy().to_string());
        args.push(workspace.input_file.to_string_lossy().to_string());

        let cmd = ExternalCommand {
            binary,
            args,
            working_dir: workspace.temp_dir.path().to_path_buf(),
            timeout: self.sandbox.timeout,
            env_clear: false,
            env: HashMap::new(),
            max_stdout_bytes: self.sandbox.max_output_bytes,
            max_stderr_bytes: self.sandbox.max_stderr_bytes,
        };

        let run = run_external_command(cmd).await.map_err(|err| {
            if err.code == "EXTERNAL_COMMAND_TIMEOUT" {
                return ConversionError::new(
                    "LIBREOFFICE_TIMEOUT",
                    "Время ожидания LibreOffice конвертации истекло.",
                );
            }
            if err.code == "EXTERNAL_BINARY_NOT_FOUND" {
                return ConversionError::new(
                    "LIBREOFFICE_NOT_AVAILABLE",
                    "LibreOffice не найден. Конвертация legacy-документа невозможна.",
                );
            }
            ConversionError::new(
                "LIBREOFFICE_CONVERSION_FAILED",
                format!("Ошибка конвертации через LibreOffice: {}", err.message),
            )
        })?;

        context.push_stage(
            ConversionStageRecord::ok("external_command_complete", self.name())
                .with_meta("duration_ms", run.duration_ms.to_string()),
        );

        let expected_ext = target.extension();
        let output_file = std::fs::read_dir(&workspace.output_dir)
            .map_err(|err| {
                ConversionError::new(
                    "LIBREOFFICE_OUTPUT_MISSING",
                    format!("Не удалось прочитать output каталог LibreOffice: {}", err),
                )
            })?
            .flatten()
            .map(|entry| entry.path())
            .find(|path| {
                path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case(expected_ext))
                    .unwrap_or(false)
            })
            .ok_or_else(|| {
                ConversionError::new(
                    "LIBREOFFICE_OUTPUT_MISSING",
                    "LibreOffice завершился без выходного файла ожидаемого формата.",
                )
            })?;

        let persistent = persist_temp_file(&output_file, "libreoffice")?;

        context.push_stage(
            ConversionStageRecord::ok("converter_cleanup", self.name())
                .with_meta("converted_path", persistent.to_string_lossy())
                .with_meta("target", target.as_str()),
        );

        Ok(ConvertedDocument {
            path: persistent,
            target,
            mime_type: target.mime_type().to_string(),
            converter_name: self.name().to_string(),
            duration_ms: run.duration_ms,
        })
    }
}
