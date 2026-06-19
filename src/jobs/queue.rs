use std::sync::Arc;

use tokio::sync::{Semaphore, mpsc};

use crate::jobs::job::Job;
use crate::jobs::worker::JobWorker;

#[derive(Clone)]
pub struct InMemoryJobQueue {
    tx: mpsc::Sender<Job>,
    max_capacity: usize,
}

impl InMemoryJobQueue {
    pub fn new(capacity: usize, max_concurrent_jobs: usize, worker: JobWorker) -> Self {
        let effective_capacity = capacity.max(1);
        let (tx, mut rx) = mpsc::channel::<Job>(effective_capacity);
        let semaphore = Arc::new(Semaphore::new(max_concurrent_jobs.max(1)));

        tokio::spawn(async move {
            while let Some(job) = rx.recv().await {
                worker.spawn_bounded(job, semaphore.clone()).await;
            }
        });

        Self {
            tx,
            max_capacity: capacity,
        }
    }

    pub async fn enqueue(&self, job: Job) -> anyhow::Result<()> {
        if self.max_capacity == 0 {
            return Err(anyhow::anyhow!("queue is full"));
        }

        self.tx
            .send(job)
            .await
            .map_err(|_| anyhow::anyhow!("queue is closed"))
    }

    pub fn capacity(&self) -> usize {
        self.max_capacity
    }

    pub fn remaining_capacity(&self) -> usize {
        if self.max_capacity == 0 {
            return 0;
        }
        self.tx.capacity()
    }

    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}
