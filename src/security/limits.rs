#[derive(Debug, Clone)]
pub struct SecurityLimits {
    pub max_file_size_mb: u64,
    pub max_pages_per_document: usize,
    pub max_extracted_assets_mb: u64,
    pub max_image_width_px: u32,
    pub max_image_height_px: u32,
    pub max_archive_entries: usize,
    pub max_archive_total_uncompressed_mb: u64,
    pub max_processing_time_sec: u64,
    pub allow_external_converters: bool,
    pub allow_network_for_converters: bool,
}

impl Default for SecurityLimits {
    fn default() -> Self {
        Self {
            max_file_size_mb: 512,
            max_pages_per_document: 5000,
            max_extracted_assets_mb: 2048,
            max_image_width_px: 10000,
            max_image_height_px: 10000,
            max_archive_entries: 10000,
            max_archive_total_uncompressed_mb: 2048,
            max_processing_time_sec: 900,
            allow_external_converters: true,
            allow_network_for_converters: false,
        }
    }
}
