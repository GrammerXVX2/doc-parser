use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobProgress {
    pub stage: Option<String>,
    pub pages_total: Option<usize>,
    pub pages_processed: Option<usize>,
    pub percent: Option<f32>,
}
