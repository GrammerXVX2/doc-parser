use std::collections::HashMap;

use serde_json::{Value, json};

#[derive(Debug, Clone, Default)]
pub struct OcrMetrics {
    pub timings_ms: HashMap<String, u64>,
    pub counters: HashMap<String, u64>,
}

impl OcrMetrics {
    pub fn set_timing(&mut self, key: &str, value: u64) {
        self.timings_ms.insert(key.to_string(), value);
    }

    pub fn set_counter(&mut self, key: &str, value: u64) {
        self.counters.insert(key.to_string(), value);
    }

    pub fn as_json(&self) -> Value {
        json!({
            "timings_ms": self.timings_ms,
            "counters": self.counters,
        })
    }
}
