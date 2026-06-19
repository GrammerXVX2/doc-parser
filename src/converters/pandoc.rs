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
pub struct PandocConverter {
    pub binary: String,
    pub enabled: bool,
    pub sandbox: SandboxOptions,
}

impl Default for PandocConverter {
    fn default() -> Self {
        Self {
            binary: "pandoc".to_string(),
            enabled: true,
            sandbox: SandboxOptions::default(),
        }
    }
}

impl PandocConverter {
    pub fn from_pipeline_config(config: Option<&PipelineConfig>) -> Self {
        let mut out = Self::default();
        out.sandbox = SandboxOptions::from_pipeline_config(config);

        let Some(config) = config else {
            return out;
        };
        let pandoc = pipeline_converters_value(config, "pandoc").unwrap_or(&Value::Null);

        out.enabled = pandoc.get("enabled").and_then(Value::as_bool).unwrap_or(true);
        out.binary = pandoc
            .get("binary")
            .and_then(Value::as_str)
            .unwrap_or("pandoc")
            .to_string();

        out
    }

    fn resolve_binary(&self) -> Result<PathBuf> {
        resolve_command_path(&self.binary).ok_or_else(|| {
            ConversionError::new(
                "PANDOC_NOT_AVAILABLE",
                "Pandoc не найден. Конвертация RTF через Pandoc невозможна.",
            )
        })
    }

    fn supports(input_ext: &str, target: ConversionTarget) -> bool {
        match (input_ext, target) {
            ("rtf", ConversionTarget::Html) => true,
            ("rtf", ConversionTarget::Markdown) => true,
            ("md", ConversionTarget::Html) => true,
            ("html", ConversionTarget::Markdown) => true,
            ("docx", ConversionTarget::Html) => true,
            _ => false,
        }
    }

    fn writer_for_target(target: ConversionTarget) -> Option<&'static str> {
        match target {
            ConversionTarget::Html => Some("html"),
            ConversionTarget::Markdown => Some("gfm"),
            _ => None,
        }
    }
}

#[async_trait]
impl DocumentConverter for PandocConverter {
    fn name(&self) -> &'static str {
        "pandoc"
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
        Self::supports(&ext, target)
    }

    async fn convert(
        &self,
        input_path: &Path,
        target: ConversionTarget,
        context: &mut ExtractionContext,
    ) -> Result<ConvertedDocument> {
        let binary = self.resolve_binary()?;
        let writer = Self::writer_for_target(target).ok_or_else(|| {
            ConversionError::new(
                "PANDOC_CONVERSION_FAILED",
                format!("Pandoc не поддерживает цель '{}'.", target.as_str()),
            )
        })?;

        let workspace = prepare_workspace(input_path, &self.sandbox, "pandoc_sandbox")?;
        let output_path = workspace.output_dir.join(format!("converted.{}", target.extension()));

        context.push_stage(
            ConversionStageRecord::ok("external_command_start", self.name())
                .with_meta("binary", binary.to_string_lossy())
                .with_meta("target", target.as_str()),
        );

        let cmd = ExternalCommand {
            binary,
            args: vec![
                workspace.input_file.to_string_lossy().to_string(),
                "-t".to_string(),
                writer.to_string(),
                "-o".to_string(),
                output_path.to_string_lossy().to_string(),
            ],
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
                    "PANDOC_TIMEOUT",
                    "Время ожидания Pandoc конвертации истекло.",
                );
            }
            if err.code == "EXTERNAL_BINARY_NOT_FOUND" {
                return ConversionError::new(
                    "PANDOC_NOT_AVAILABLE",
                    "Pandoc не найден. Конвертация RTF через Pandoc невозможна.",
                );
            }
            ConversionError::new(
                "PANDOC_CONVERSION_FAILED",
                format!("Ошибка конвертации через Pandoc: {}", err.message),
            )
        })?;

        context.push_stage(
            ConversionStageRecord::ok("external_command_complete", self.name())
                .with_meta("duration_ms", run.duration_ms.to_string()),
        );

        if !output_path.exists() {
            return Err(ConversionError::new(
                "PANDOC_OUTPUT_MISSING",
                "Pandoc завершился без ожидаемого выходного файла.",
            ));
        }

        let persistent = persist_temp_file(&output_path, "pandoc")?;
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
