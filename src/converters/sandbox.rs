use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use crate::config::{PipelineConfig, pipeline_converters_value};
use crate::converters::traits::{ConversionError, Result};
use crate::utils::tempdir::{TempDirGuard, copy_file_to_dir, create_isolated_temp_dir};

#[derive(Debug, Clone)]
pub struct SandboxOptions {
    pub enabled: bool,
    pub timeout: Duration,
    pub max_output_bytes: usize,
    pub max_stderr_bytes: usize,
    pub use_isolated_temp_dir: bool,
    pub disable_network_best_effort: bool,
    pub cleanup_temp_dir: bool,
}

impl Default for SandboxOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout: Duration::from_secs(120),
            max_output_bytes: 512 * 1024 * 1024,
            max_stderr_bytes: 10 * 1024 * 1024,
            use_isolated_temp_dir: true,
            disable_network_best_effort: true,
            cleanup_temp_dir: true,
        }
    }
}

impl SandboxOptions {
    pub fn from_pipeline_config(config: Option<&PipelineConfig>) -> Self {
        let mut out = Self::default();
        let Some(config) = config else {
            return out;
        };

        let Some(sandbox) = pipeline_converters_value(config, "sandbox") else {
            return out;
        };
        let sandbox = sandbox.get("sandbox").unwrap_or(sandbox);

        out.enabled = sandbox.get("enabled").and_then(Value::as_bool).unwrap_or(out.enabled);
        out.timeout = Duration::from_secs(
            sandbox
                .get("timeout_sec")
                .and_then(Value::as_u64)
                .unwrap_or(out.timeout.as_secs()),
        );
        out.max_output_bytes = sandbox
            .get("max_output_size_mb")
            .and_then(Value::as_u64)
            .map(|v| (v as usize) * 1024 * 1024)
            .unwrap_or(out.max_output_bytes);
        out.max_stderr_bytes = sandbox
            .get("max_stderr_size_mb")
            .and_then(Value::as_u64)
            .map(|v| (v as usize) * 1024 * 1024)
            .unwrap_or(out.max_stderr_bytes);
        out.use_isolated_temp_dir = sandbox
            .get("use_isolated_temp_dir")
            .and_then(Value::as_bool)
            .unwrap_or(out.use_isolated_temp_dir);
        out.disable_network_best_effort = sandbox
            .get("disable_network_best_effort")
            .and_then(Value::as_bool)
            .unwrap_or(out.disable_network_best_effort);
        out.cleanup_temp_dir = sandbox
            .get("cleanup_temp_dir")
            .and_then(Value::as_bool)
            .unwrap_or(out.cleanup_temp_dir);

        out
    }
}

#[derive(Debug)]
pub struct SandboxWorkspace {
    pub temp_dir: TempDirGuard,
    pub input_file: std::path::PathBuf,
    pub output_dir: std::path::PathBuf,
}

pub fn prepare_workspace(input_path: &Path, options: &SandboxOptions, prefix: &str) -> Result<SandboxWorkspace> {
    if !options.use_isolated_temp_dir {
        return Err(ConversionError::new(
            "EXTERNAL_TEMP_DIR_FAILED",
            "Sandbox без изолированного временного каталога не поддерживается.",
        ));
    }

    let temp_dir = create_isolated_temp_dir(prefix, options.cleanup_temp_dir)?;
    let input_file = copy_file_to_dir(input_path, temp_dir.path())?;
    let output_dir = temp_dir.path().join("out");
    std::fs::create_dir_all(&output_dir).map_err(|err| {
        ConversionError::new(
            "EXTERNAL_TEMP_DIR_FAILED",
            format!("Не удалось создать каталог output в sandbox: {}", err),
        )
    })?;

    Ok(SandboxWorkspace {
        temp_dir,
        input_file,
        output_dir,
    })
}
