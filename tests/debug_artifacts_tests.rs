use std::fs;
use std::path::PathBuf;

use document_parser::assets::LocalAssetStore;
use document_parser::debug::write_debug_json_asset;

#[test]
fn debug_layout_json_written_and_registered() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug_artifacts_tests");
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(&out).expect("mkdir");

    let store = LocalAssetStore::new(&out);
    let asset = write_debug_json_asset(
        &store,
        "doc_debug_test",
        "page_1_layout_regions.json",
        &serde_json::json!([{"type":"table"}]),
    )
    .expect("write");

    assert_eq!(asset.asset_type, "debug");
    let full = out.join("doc_debug_test").join(asset.path);
    assert!(full.exists());
}
