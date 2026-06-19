use std::fs;
use std::path::{Path, PathBuf};

use crate::converters::traits::{ConversionError, Result};

#[derive(Debug)]
pub struct TempDirGuard {
    path: PathBuf,
    cleanup: bool,
}

impl TempDirGuard {
    pub fn new(path: PathBuf, cleanup: bool) -> Self {
        Self { path, cleanup }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        if self.cleanup {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

pub fn create_isolated_temp_dir(prefix: &str, cleanup: bool) -> Result<TempDirGuard> {
    let dir = std::env::temp_dir().join(format!("{}_{}", prefix, uuid::Uuid::new_v4().simple()));
    fs::create_dir_all(&dir).map_err(|err| {
        ConversionError::new(
            "EXTERNAL_TEMP_DIR_FAILED",
            format!("Не удалось создать временный каталог sandbox: {}", err),
        )
    })?;
    Ok(TempDirGuard::new(dir, cleanup))
}

pub fn copy_file_to_dir(input_path: &Path, target_dir: &Path) -> Result<PathBuf> {
    let file_name = input_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            ConversionError::new(
                "EXTERNAL_TEMP_DIR_FAILED",
                "Не удалось определить имя входного файла для sandbox.".to_string(),
            )
        })?;
    let target_path = target_dir.join(file_name);
    fs::copy(input_path, &target_path).map_err(|err| {
        ConversionError::new(
            "EXTERNAL_TEMP_DIR_FAILED",
            format!("Не удалось скопировать входной файл в sandbox: {}", err),
        )
    })?;
    Ok(target_path)
}

pub fn persist_temp_file(file_path: &Path, namespace: &str) -> Result<PathBuf> {
    let root = std::env::temp_dir().join("document_parser_converted").join(namespace);
    fs::create_dir_all(&root).map_err(|err| {
        ConversionError::new(
            "EXTERNAL_TEMP_DIR_FAILED",
            format!("Не удалось создать каталог для конвертированных файлов: {}", err),
        )
    })?;

    let file_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            ConversionError::new(
                "EXTERNAL_TEMP_DIR_FAILED",
                "Не удалось определить имя конвертированного файла.".to_string(),
            )
        })?;

    let output = root.join(format!("{}_{}", uuid::Uuid::new_v4().simple(), file_name));
    fs::copy(file_path, &output).map_err(|err| {
        ConversionError::new(
            "EXTERNAL_TEMP_DIR_FAILED",
            format!("Не удалось сохранить конвертированный файл вне sandbox: {}", err),
        )
    })?;

    Ok(output)
}
