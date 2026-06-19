pub fn content_type_for_media_path(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".bmp") {
        "image/bmp"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".tif") || lower.ends_with(".tiff") {
        "image/tiff"
    } else {
        "application/octet-stream"
    }
}

pub fn list_media_entries(entries: &[String], prefix: &str) -> Vec<String> {
    let mut out = entries
        .iter()
        .filter(|e| e.starts_with(prefix))
        .cloned()
        .collect::<Vec<_>>();
    out.sort();
    out
}

pub fn normalize_relationship_target(base_dir: &str, target: &str) -> String {
    if target.starts_with('/') {
        return target.trim_start_matches('/').to_string();
    }

    let mut parts = base_dir
        .trim_end_matches('/')
        .split('/')
        .filter(|p| !p.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    for seg in target.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            if !parts.is_empty() {
                parts.pop();
            }
            continue;
        }
        parts.push(seg.to_string());
    }

    parts.join("/")
}
