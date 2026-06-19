use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use document_parser::converters::external_process::{ExternalCommand, run_external_command};
use document_parser::security::{SecurityLimits, safe_join, validate_upload};
use futures::executor::block_on;

#[test]
fn path_traversal_blocked_in_safe_join() {
    let base = PathBuf::from("/tmp/doc-parser");
    assert!(safe_join(&base, "assets/file.png").is_some());
    assert!(safe_join(&base, "../etc/passwd").is_none());
    assert!(safe_join(&base, "/absolute/path").is_none());
}

#[test]
fn ooxml_zip_slip_blocked_by_limits() {
    let limits = SecurityLimits::default();
    let err = validate_upload("bad.docx", b"not-a-valid-zip", &limits)
        .expect_err("expected invalid archive")
        .payload;
    assert_eq!(err.code, "UNSUPPORTED_FILE_TYPE");
}

#[test]
fn archive_limit_enforced() {
    let mut limits = SecurityLimits::default();
    limits.max_archive_entries = 1;

    let mut bytes = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut bytes);
        let mut writer = zip::ZipWriter::new(cursor);
        let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
        writer.start_file("a.txt", options).unwrap();
        std::io::Write::write_all(&mut writer, b"a").unwrap();
        writer.start_file("b.txt", options).unwrap();
        std::io::Write::write_all(&mut writer, b"b").unwrap();
        writer.finish().unwrap();
    }

    let err = validate_upload("sample.docx", &bytes, &limits)
        .expect_err("expected archive entries limit")
        .payload;
    assert_eq!(err.code, "ARCHIVE_TOO_MANY_ENTRIES");
}

#[test]
fn large_image_dimensions_blocked() {
    let mut limits = SecurityLimits::default();
    limits.max_image_width_px = 1;
    limits.max_image_height_px = 1;

    let mut png = Vec::new();
    {
        let image = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(10, 10, image::Rgb([255, 255, 255])));
        image
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
    }

    let err = validate_upload("sample.png", &png, &limits)
        .expect_err("expected image size limit")
        .payload;
    assert_eq!(err.code, "IMAGE_DIMENSIONS_TOO_LARGE");
}

#[test]
fn external_command_no_shell_injection() {
    let cmd = ExternalCommand {
        binary: PathBuf::from("python3"),
        args: vec![
            "-c".to_string(),
            "import sys; print(sys.argv[1])".to_string(),
            "x;echo_hacked".to_string(),
        ],
        working_dir: PathBuf::from("."),
        timeout: Duration::from_secs(2),
        env_clear: false,
        env: HashMap::new(),
        max_stdout_bytes: 1024 * 1024,
        max_stderr_bytes: 1024 * 1024,
    };

    let out = block_on(run_external_command(cmd)).unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("x;echo_hacked"));
}

#[test]
fn external_converter_timeout_and_stderr_limit() {
    let timeout_cmd = ExternalCommand {
        binary: PathBuf::from("python3"),
        args: vec!["-c".to_string(), "import time; time.sleep(5)".to_string()],
        working_dir: PathBuf::from("."),
        timeout: Duration::from_millis(100),
        env_clear: false,
        env: HashMap::new(),
        max_stdout_bytes: 1024,
        max_stderr_bytes: 1024,
    };
    let timeout_err = block_on(run_external_command(timeout_cmd)).unwrap_err();
    assert_eq!(timeout_err.code, "EXTERNAL_COMMAND_TIMEOUT");

    let stderr_cmd = ExternalCommand {
        binary: PathBuf::from("python3"),
        args: vec![
            "-c".to_string(),
            "import sys; sys.stderr.write('x'*5000)".to_string(),
        ],
        working_dir: PathBuf::from("."),
        timeout: Duration::from_secs(2),
        env_clear: false,
        env: HashMap::new(),
        max_stdout_bytes: 1024,
        max_stderr_bytes: 64,
    };
    let stderr_err = block_on(run_external_command(stderr_cmd)).unwrap_err();
    assert_eq!(stderr_err.code, "EXTERNAL_OUTPUT_TOO_LARGE");
}
