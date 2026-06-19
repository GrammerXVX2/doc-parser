use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerPoolConfig {
    pub io_workers: usize,
    pub cpu_workers: usize,
    pub ocr_detection_workers: usize,
    pub ocr_recognition_workers: usize,
    pub layout_workers: usize,
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            io_workers: 4,
            cpu_workers: 0,
            ocr_detection_workers: 1,
            ocr_recognition_workers: 1,
            layout_workers: 1,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct WorkerPools {
    pub config: WorkerPoolConfig,
}

impl WorkerPools {
    pub fn new(config: WorkerPoolConfig) -> Self {
        Self { config }
    }
}
