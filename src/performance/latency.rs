use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Default)]
pub struct LatencyTracker {
    samples_ms: Arc<Mutex<Vec<f64>>>,
}

#[derive(Debug, Clone, Default)]
pub struct LatencySummary {
    pub count: usize,
    pub min_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

impl LatencyTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&self, value_ms: f64) {
        if value_ms.is_finite() && value_ms >= 0.0 {
            if let Ok(mut samples) = self.samples_ms.lock() {
                samples.push(value_ms);
            }
        }
    }

    pub fn summary(&self) -> LatencySummary {
        let Ok(samples) = self.samples_ms.lock() else {
            return LatencySummary::default();
        };

        if samples.is_empty() {
            return LatencySummary::default();
        }

        let mut sorted = samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let count = sorted.len();
        let sum: f64 = sorted.iter().sum();
        LatencySummary {
            count,
            min_ms: sorted[0],
            max_ms: sorted[count - 1],
            mean_ms: sum / count as f64,
            p50_ms: percentile(&sorted, 0.50),
            p95_ms: percentile(&sorted, 0.95),
            p99_ms: percentile(&sorted, 0.99),
        }
    }
}

fn percentile(sorted_samples: &[f64], quantile: f64) -> f64 {
    if sorted_samples.is_empty() {
        return 0.0;
    }

    let q = quantile.clamp(0.0, 1.0);
    let rank = ((sorted_samples.len() - 1) as f64 * q).round() as usize;
    sorted_samples[rank]
}
