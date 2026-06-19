use std::fs;

use document_parser::assets::{AssetStore, AssetType, LocalAssetStore};

#[test]
fn local_asset_store_writes_asset_and_hash() {
    let root = std::env::temp_dir().join(format!(
        "document_parser_asset_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);

    let store = LocalAssetStore::new(&root);
    let asset = store
        .write_asset(
            "doc1",
            AssetType::EmbeddedImage,
            "sample.png",
            b"abc123",
            "image/png",
        )
        .expect("asset write should succeed");

    assert!(!asset.asset_id.is_empty());
    assert!(asset.sha256.is_some());
    let full_path = root.join("doc1").join(asset.path);
    assert!(full_path.exists());
}
