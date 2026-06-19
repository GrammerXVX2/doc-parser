use std::path::Path;

#[test]
fn dev_launch_artifacts_exist() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let required = [
        "scripts/dev_smoke_api.sh",
        "scripts/dev_clean_data.sh",
        "docker-compose.dev.yml",
        "docs/DEV_LAUNCH.md",
    ];

    for path in required {
        assert!(
            root.join(path).exists(),
            "required artifact is missing: {path}"
        );
    }
}
