use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crate::converters::traits::{ConversionError, Result};
use crate::utils::process::{bytes_to_mb, truncate_bytes};

#[derive(Debug, Clone)]
pub struct ExternalCommand {
    pub binary: PathBuf,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
    pub timeout: Duration,
    pub env_clear: bool,
    pub env: HashMap<String, String>,
    pub max_stdout_bytes: usize,
    pub max_stderr_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct ExternalCommandOutput {
    pub status_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub duration_ms: u64,
    pub timed_out: bool,
}

pub async fn run_external_command(cmd: ExternalCommand) -> Result<ExternalCommandOutput> {
    run_external_command_blocking(cmd)
}

fn run_external_command_blocking(cmd: ExternalCommand) -> Result<ExternalCommandOutput> {
    if cmd.timeout.is_zero() {
        return Err(ConversionError::new(
            "EXTERNAL_COMMAND_TIMEOUT",
            "Не задан timeout для внешней команды.",
        ));
    }

    let start = Instant::now();
    let binary_for_error = cmd.binary.clone();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut command = Command::new(&cmd.binary);
        command.args(&cmd.args);
        command.current_dir(&cmd.working_dir);

        if cmd.env_clear {
            command.env_clear();
        }
        for (key, value) in cmd.env {
            command.env(key, value);
        }

        let output = command.output();
        let _ = tx.send(output);
    });

    let output = match rx.recv_timeout(cmd.timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            return Err(ConversionError::new(
                "EXTERNAL_COMMAND_TIMEOUT",
                "Превышено время ожидания выполнения внешней команды.",
            )
            .with_meta("timeout_sec", cmd.timeout.as_secs().to_string()));
        }
        Err(err) => {
            return Err(ConversionError::new(
                "EXTERNAL_COMMAND_FAILED",
                format!("Ошибка взаимодействия с внешней командой: {}", err),
            ));
        }
    };

    let output = match output {
        Ok(v) => v,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(ConversionError::new(
                "EXTERNAL_BINARY_NOT_FOUND",
                format!(
                    "Внешний инструмент не найден: {}",
                    binary_for_error.to_string_lossy()
                ),
            )
            .with_meta("binary", binary_for_error.to_string_lossy()));
        }
        Err(err) => {
            return Err(ConversionError::new(
                "EXTERNAL_COMMAND_FAILED",
                format!("Не удалось запустить внешнюю команду: {}", err),
            ));
        }
    };

    let (stdout, stdout_truncated) = truncate_bytes(output.stdout, cmd.max_stdout_bytes);
    let (stderr, stderr_truncated) = truncate_bytes(output.stderr, cmd.max_stderr_bytes);

    if stdout_truncated {
        return Err(ConversionError::new(
            "EXTERNAL_OUTPUT_TOO_LARGE",
            format!(
                "Stdout внешней команды превысил лимит {:.2} MB и был обрезан.",
                bytes_to_mb(cmd.max_stdout_bytes)
            ),
        ));
    }
    if stderr_truncated {
        return Err(ConversionError::new(
            "EXTERNAL_OUTPUT_TOO_LARGE",
            format!(
                "Stderr внешней команды превысил лимит {:.2} MB и был обрезан.",
                bytes_to_mb(cmd.max_stderr_bytes)
            ),
        ));
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let status_code = output.status.code();

    if !output.status.success() {
        let stderr_text = String::from_utf8_lossy(&stderr);
        return Err(ConversionError::new(
            "EXTERNAL_COMMAND_FAILED",
            format!(
                "Внешняя команда завершилась с ошибкой (код {:?}): {}",
                status_code,
                stderr_text.trim()
            ),
        )
        .with_meta("status_code", status_code.unwrap_or(-1).to_string()));
    }

    Ok(ExternalCommandOutput {
        status_code,
        stdout,
        stderr,
        duration_ms,
        timed_out: false,
    })
}
