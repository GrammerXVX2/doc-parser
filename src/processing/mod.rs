use std::time::Instant;

pub struct ProcessingTimer {
    started: Instant,
}

impl ProcessingTimer {
    pub fn start() -> Self {
        Self {
            started: Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }
}
