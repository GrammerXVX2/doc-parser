use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::utils::zip::is_safe_zip_entry_path;

const MAX_ENTRY_BYTES: usize = 32 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct OoxmlPackage {
    pub path: PathBuf,
    pub entries: HashMap<String, Vec<u8>>,
}

impl OoxmlPackage {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let file = std::fs::File::open(path).with_context(|| {
            format!("OOXML_ZIP_OPEN_FAILED: failed to open package {}", path.display())
        })?;
        let mut archive = zip::ZipArchive::new(file).with_context(|| {
            format!("OOXML_ZIP_OPEN_FAILED: invalid zip package {}", path.display())
        })?;

        let mut entries = HashMap::new();
        let mut total_bytes = 0usize;

        for idx in 0..archive.len() {
            let mut entry = archive.by_index(idx).with_context(|| {
                format!("OOXML_INVALID_PACKAGE: failed to read zip entry #{idx}")
            })?;
            let name = entry.name().to_string();
            if !is_safe_zip_entry_path(&name) {
                anyhow::bail!(
                    "OOXML_PATH_TRAVERSAL_BLOCKED: unsafe entry path '{}' in {}",
                    name,
                    path.display()
                );
            }

            let entry_size = entry.size() as usize;
            if entry_size > MAX_ENTRY_BYTES {
                anyhow::bail!(
                    "OOXML_ENTRY_TOO_LARGE: '{}' exceeds {} bytes",
                    name,
                    MAX_ENTRY_BYTES
                );
            }
            total_bytes += entry_size;
            if total_bytes > MAX_TOTAL_BYTES {
                anyhow::bail!(
                    "OOXML_ZIP_BOMB_DETECTED: package exceeds {} bytes",
                    MAX_TOTAL_BYTES
                );
            }

            let mut bytes = Vec::with_capacity(entry_size);
            entry.read_to_end(&mut bytes).with_context(|| {
                format!("OOXML_INVALID_PACKAGE: failed to read '{}'", name)
            })?;
            entries.insert(name, bytes);
        }

        Ok(Self {
            path: path.to_path_buf(),
            entries,
        })
    }

    pub fn read_text(&self, entry: &str) -> anyhow::Result<Option<String>> {
        match self.entries.get(entry) {
            Some(bytes) => {
                let text = std::str::from_utf8(bytes).with_context(|| {
                    format!("OOXML_INVALID_PACKAGE: '{}' is not valid UTF-8", entry)
                })?;
                Ok(Some(text.to_string()))
            }
            None => Ok(None),
        }
    }

    pub fn read_bytes(&self, entry: &str) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(self.entries.get(entry).cloned())
    }

    pub fn list_entries(&self) -> Vec<String> {
        let mut out = self.entries.keys().cloned().collect::<Vec<_>>();
        out.sort();
        out
    }
}
