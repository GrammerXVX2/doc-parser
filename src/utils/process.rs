pub fn truncate_bytes(mut data: Vec<u8>, max_bytes: usize) -> (Vec<u8>, bool) {
    if max_bytes == 0 {
        return (Vec::new(), !data.is_empty());
    }
    if data.len() <= max_bytes {
        return (data, false);
    }

    data.truncate(max_bytes);
    (data, true)
}

pub fn bytes_to_mb(bytes: usize) -> f32 {
    bytes as f32 / (1024.0 * 1024.0)
}
