use std::env;
use std::path::{Path, PathBuf};

pub fn resolve_command_path(command: &str) -> Option<PathBuf> {
    if command.trim().is_empty() {
        return None;
    }

    let candidate = Path::new(command);
    if candidate.components().count() > 1 {
        if candidate.exists() {
            return Some(candidate.to_path_buf());
        }
        return None;
    }

    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let full = dir.join(command);
        if full.is_file() {
            return Some(full);
        }
    }

    None
}

pub fn command_exists(command: &str) -> bool {
    resolve_command_path(command).is_some()
}
