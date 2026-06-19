pub fn is_safe_zip_entry_path(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    if name.starts_with('/') || name.starts_with('\\') {
        return false;
    }
    if name.contains("..") {
        return false;
    }
    if name.contains(':') {
        return false;
    }
    true
}
