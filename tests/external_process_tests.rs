use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use document_parser::converters::external_process::{ExternalCommand, run_external_command};
use futures::executor::block_on;

fn base_cmd(binary: &str, args: Vec<String>) -> ExternalCommand {
    ExternalCommand {
        binary: PathBuf::from(binary),
        args,
        working_dir: PathBuf::from("."),
        timeout: Duration::from_secs(2),
        env_clear: false,
        env: HashMap::new(),
        max_stdout_bytes: 1024 * 1024,
        max_stderr_bytes: 1024 * 1024,
    }
}

#[test]
fn command_not_found_returns_structured_error() {
    let err = block_on(run_external_command(base_cmd("definitely_missing_binary_xyz", vec![])))
        .expect_err("expected missing binary error");
    assert_eq!(err.code, "EXTERNAL_BINARY_NOT_FOUND");
}

#[test]
fn timeout_returns_structured_error() {
    let mut cmd = base_cmd(
        "python3",
        vec!["-c".to_string(), "import time; time.sleep(5)".to_string()],
    );
    cmd.timeout = Duration::from_millis(200);

    let err = block_on(run_external_command(cmd)).expect_err("expected timeout");
    assert_eq!(err.code, "EXTERNAL_COMMAND_TIMEOUT");
}

#[test]
fn stdout_and_stderr_are_captured() {
    let cmd = base_cmd(
        "python3",
        vec![
            "-c".to_string(),
            "import sys;print('out');print('err', file=sys.stderr)".to_string(),
        ],
    );

    let out = block_on(run_external_command(cmd)).expect("command should pass");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(stdout.contains("out"));
    assert!(stderr.contains("err"));
}

#[test]
fn args_are_passed_without_shell_interpolation() {
    let cmd = base_cmd(
        "python3",
        vec![
            "-c".to_string(),
            "import sys; print(sys.argv[1])".to_string(),
            "x;echo_hacked".to_string(),
        ],
    );

    let out = block_on(run_external_command(cmd)).expect("command should pass");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("x;echo_hacked"));
}

#[test]
fn too_large_stderr_returns_structured_error() {
    let mut cmd = base_cmd(
        "python3",
        vec![
            "-c".to_string(),
            "import sys; sys.stderr.write('x'*5000)".to_string(),
        ],
    );
    cmd.max_stderr_bytes = 128;

    let err = block_on(run_external_command(cmd)).expect_err("expected output too large");
    assert_eq!(err.code, "EXTERNAL_OUTPUT_TOO_LARGE");
}
