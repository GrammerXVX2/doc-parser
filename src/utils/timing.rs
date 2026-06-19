use std::time::Instant;

#[derive(Debug, Clone)]
pub struct StageTimer {
    started_at: Instant,
}

impl StageTimer {
    pub fn start() -> Self {
        Self {
            started_at: Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }
}
