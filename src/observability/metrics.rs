use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

#[derive(Debug, Default)]
pub struct MetricsRegistry {
    counters: Arc<RwLock<HashMap<String, u64>>>,
    histograms: Arc<RwLock<HashMap<String, Vec<f64>>>>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn inc(&self, key: &str) {
        let mut counters = self.counters.write().await;
        *counters.entry(key.to_string()).or_insert(0) += 1;
    }

    pub async fn add(&self, key: &str, value: u64) {
        let mut counters = self.counters.write().await;
        *counters.entry(key.to_string()).or_insert(0) += value;
    }

    pub async fn observe_ms(&self, key: &str, value: f64) {
        let mut histograms = self.histograms.write().await;
        histograms.entry(key.to_string()).or_default().push(value);
    }

    pub async fn render_prometheus(&self) -> String {
        let counters = self.counters.read().await;
        let histograms = self.histograms.read().await;

        let mut out = String::new();
        for (name, value) in counters.iter() {
            out.push_str(&format!("# TYPE {} counter\n{} {}\n", name, name, value));
        }

        for (name, values) in histograms.iter() {
            let sum: f64 = values.iter().sum();
            let count = values.len();
            out.push_str(&format!("# TYPE {} summary\n", name));
            out.push_str(&format!("{}_sum {}\n", name, sum));
            out.push_str(&format!("{}_count {}\n", name, count));
        }

        out
    }
}
