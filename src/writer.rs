use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::model::DocumentModel;

pub fn write_model_json(model: &DocumentModel, output_dir: &Path, pretty: bool) -> anyhow::Result<PathBuf> {
    let doc_dir = output_dir.join(&model.document_id);
    fs::create_dir_all(&doc_dir)
        .with_context(|| format!("failed to create output directory: {}", doc_dir.display()))?;

    let output_path = doc_dir.join("model.json");
    let bytes = if pretty {
        serde_json::to_vec_pretty(model)?
    } else {
        serde_json::to_vec(model)?
    };

    fs::write(&output_path, bytes)
        .with_context(|| format!("failed to write model json: {}", output_path.display()))?;

    Ok(output_path)
}

pub fn write_document_outputs(model: &DocumentModel, output_dir: &Path, pretty: bool) -> anyhow::Result<PathBuf> {
    let model_path = write_model_json(model, output_dir, pretty)?;
    let doc_dir = model_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("failed to get document output directory"))?
        .to_path_buf();

    let markdown = model
        .pages
        .iter()
        .map(|page| page.markdown.as_str())
        .filter(|v| !v.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    let plain_text = model
        .pages
        .iter()
        .map(|page| page.text.as_str())
        .filter(|v| !v.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    fs::write(doc_dir.join("markdown.md"), markdown)
        .with_context(|| "failed to write markdown.md")?;
    fs::write(doc_dir.join("plain_text.txt"), plain_text)
        .with_context(|| "failed to write plain_text.txt")?;

    Ok(model_path)
}
