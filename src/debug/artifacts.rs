use std::fs;
use std::path::Path;

use crate::assets::{AssetStore, AssetType};
use crate::model::Asset;

pub fn write_debug_json_asset(
    store: &dyn AssetStore,
    document_id: &str,
    suggested_name: &str,
    value: &serde_json::Value,
) -> anyhow::Result<Asset> {
    let bytes = serde_json::to_vec_pretty(value)?;
    store.write_asset(
        document_id,
        AssetType::Debug,
        suggested_name,
        &bytes,
        "application/json",
    )
}

pub fn write_debug_json_file(path: &Path, value: &serde_json::Value) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
