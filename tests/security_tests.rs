use document_parser::security::{SecurityLimits, safe_join, sanitize_filename, validate_upload};

#[test]
fn safe_filename_and_path_traversal_blocked() {
    assert!(sanitize_filename("../../passwd").is_some());
    let base = std::path::PathBuf::from("data/output/doc");
    assert!(safe_join(&base, "assets/images/a.png").is_some());
    assert!(safe_join(&base, "../evil").is_none());
}

#[test]
fn file_size_limit_and_unsupported_type_blocked() {
    let mut limits = SecurityLimits::default();
    limits.max_file_size_mb = 1;

    let huge = vec![0_u8; 2 * 1024 * 1024];
    let too_large = validate_upload("sample.md", &huge, &limits);
    assert!(too_large.is_err());

    let unsupported = validate_upload("sample.exe", b"abc", &limits);
    assert!(unsupported.is_err());
}
