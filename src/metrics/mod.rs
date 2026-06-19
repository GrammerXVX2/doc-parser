use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct PipelineMetrics {
    counters: HashMap<String, u64>,
    durations_ms: HashMap<String, Vec<u64>>,
}

impl PipelineMetrics {
    pub fn inc(&mut self, name: &str) {
        *self.counters.entry(name.to_string()).or_insert(0) += 1;
    }

    pub fn observe_duration(&mut self, stage: &str, duration_ms: u64) {
        self.durations_ms
            .entry(stage.to_string())
            .or_default()
            .push(duration_ms);
    }

    pub fn snapshot(&self) -> HashMap<String, u64> {
        self.counters.clone()
    }
}
