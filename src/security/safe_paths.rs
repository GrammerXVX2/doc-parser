use std::path::{Component, Path, PathBuf};

pub fn sanitize_filename(filename: &str) -> Option<String> {
    let trimmed = filename.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }

    if out.is_empty() || out == "." || out == ".." {
        None
    } else {
        Some(out)
    }
}

pub fn safe_join(base: &Path, relative: &str) -> Option<PathBuf> {
    let candidate = Path::new(relative);
    if candidate.is_absolute() {
        return None;
    }
    let mut out = PathBuf::from(base);
    for component in candidate.components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            _ => return None,
        }
    }
    Some(out)
}
