use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot, watch};

use crate::performance::latency::LatencyTracker;

#[derive(Debug)]
pub struct BatchRequest<T, R> {
    pub payload: T,
    pub response_tx: oneshot::Sender<anyhow::Result<R>>,
    pub enqueued_at: Instant,
}

#[async_trait]
pub trait BatchProcessor<T, R>: Send + Sync
where
    T: Send + 'static,
    R: Send + 'static,
{
    async fn process(&self, batch: Vec<T>) -> anyhow::Result<Vec<R>>;
}

#[derive(Debug, Clone, Default)]
pub struct BatcherSnapshot {
    pub submitted_total: u64,
    pub batches_total: u64,
    pub batch_timeouts_total: u64,
    pub responses_dropped_total: u64,
    pub queue_depth: usize,
    pub avg_batch_size: f64,
    pub avg_queue_wait_ms: f64,
    pub avg_inference_ms: f64,
}

#[derive(Debug, Default)]
struct BatcherStats {
    submitted_total: AtomicU64,
    batches_total: AtomicU64,
    batch_timeouts_total: AtomicU64,
    responses_dropped_total: AtomicU64,
    queue_depth: AtomicUsize,
    processed_items_total: AtomicU64,
    queue_wait_ms_total: AtomicU64,
    inference_ms_total: AtomicU64,
    queue_wait_tracker: LatencyTracker,
    inference_tracker: LatencyTracker,
}

pub struct AsyncBatcher<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    pub max_batch_size: usize,
    pub max_wait: Duration,
    tx: mpsc::Sender<BatchRequest<T, R>>,
    shutdown_tx: watch::Sender<bool>,
    stats: Arc<BatcherStats>,
}

impl<T, R> Clone for AsyncBatcher<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            max_batch_size: self.max_batch_size,
            max_wait: self.max_wait,
            tx: self.tx.clone(),
            shutdown_tx: self.shutdown_tx.clone(),
            stats: self.stats.clone(),
        }
    }
}

impl<T, R> AsyncBatcher<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    pub fn new(
        max_batch_size: usize,
        max_wait: Duration,
        processor: Arc<dyn BatchProcessor<T, R>>,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<BatchRequest<T, R>>(max_batch_size.max(1) * 32);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let stats = Arc::new(BatcherStats::default());

        tokio::spawn(worker_loop(
            rx,
            processor,
            max_batch_size.max(1),
            max_wait,
            shutdown_rx,
            stats.clone(),
        ));

        Self {
            max_batch_size: max_batch_size.max(1),
            max_wait,
            tx,
            shutdown_tx,
            stats,
        }
    }

    pub async fn enqueue(&self, payload: T) -> anyhow::Result<R> {
        let response_rx = self.submit(payload).await?;

        response_rx
            .await
            .map_err(|_| anyhow::anyhow!("BATCHER_RESPONSE_DROPPED: response channel closed"))?
    }

    pub async fn submit(&self, payload: T) -> anyhow::Result<oneshot::Receiver<anyhow::Result<R>>> {
        let (response_tx, response_rx) = oneshot::channel();
        let request = BatchRequest {
            payload,
            response_tx,
            enqueued_at: Instant::now(),
        };

        self.stats.submitted_total.fetch_add(1, Ordering::Relaxed);
        self.stats.queue_depth.fetch_add(1, Ordering::Relaxed);

        self.tx
            .send(request)
            .await
            .map_err(|_| anyhow::anyhow!("BATCHER_SHUTDOWN: batcher is not accepting new requests"))?;

        Ok(response_rx)
    }

    pub fn close(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    pub fn snapshot(&self) -> BatcherSnapshot {
        let submitted_total = self.stats.submitted_total.load(Ordering::Relaxed);
        let batches_total = self.stats.batches_total.load(Ordering::Relaxed);
        let processed_items_total = self.stats.processed_items_total.load(Ordering::Relaxed);
        let queue_wait_ms_total = self.stats.queue_wait_ms_total.load(Ordering::Relaxed);
        let inference_ms_total = self.stats.inference_ms_total.load(Ordering::Relaxed);

        let avg_batch_size = if batches_total == 0 {
            0.0
        } else {
            processed_items_total as f64 / batches_total as f64
        };

        let avg_queue_wait_ms = if processed_items_total == 0 {
            0.0
        } else {
            queue_wait_ms_total as f64 / processed_items_total as f64
        };

        let avg_inference_ms = if batches_total == 0 {
            0.0
        } else {
            inference_ms_total as f64 / batches_total as f64
        };

        BatcherSnapshot {
            submitted_total,
            batches_total,
            batch_timeouts_total: self.stats.batch_timeouts_total.load(Ordering::Relaxed),
            responses_dropped_total: self.stats.responses_dropped_total.load(Ordering::Relaxed),
            queue_depth: self.stats.queue_depth.load(Ordering::Relaxed),
            avg_batch_size,
            avg_queue_wait_ms,
            avg_inference_ms,
        }
    }

    pub fn queue_wait_summary(&self) -> crate::performance::latency::LatencySummary {
        self.stats.queue_wait_tracker.summary()
    }

    pub fn inference_summary(&self) -> crate::performance::latency::LatencySummary {
        self.stats.inference_tracker.summary()
    }
}

async fn worker_loop<T, R>(
    mut rx: mpsc::Receiver<BatchRequest<T, R>>,
    processor: Arc<dyn BatchProcessor<T, R>>,
    max_batch_size: usize,
    max_wait: Duration,
    mut shutdown_rx: watch::Receiver<bool>,
    stats: Arc<BatcherStats>,
) where
    T: Send + 'static,
    R: Send + 'static,
{
    let mut pending: Vec<BatchRequest<T, R>> = Vec::with_capacity(max_batch_size);
    let mut first_enqueued_at: Option<Instant> = None;

    loop {
        if pending.is_empty() {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    if changed.is_ok() && *shutdown_rx.borrow() {
                        break;
                    }
                }
                maybe_req = rx.recv() => {
                    let Some(req) = maybe_req else {
                        break;
                    };
                    first_enqueued_at = Some(req.enqueued_at);
                    pending.push(req);
                }
            }
        }

        while !pending.is_empty() {
            if pending.len() >= max_batch_size {
                flush_batch(&mut pending, &processor, &stats).await;
                first_enqueued_at = None;
                continue;
            }

            let wait_deadline = first_enqueued_at
                .map(|t| t + max_wait)
                .unwrap_or_else(|| Instant::now() + max_wait);
            let sleep = tokio::time::sleep_until(tokio::time::Instant::from_std(wait_deadline));
            tokio::pin!(sleep);

            tokio::select! {
                changed = shutdown_rx.changed() => {
                    if changed.is_ok() && *shutdown_rx.borrow() {
                        flush_batch(&mut pending, &processor, &stats).await;
                        return;
                    }
                }
                maybe_req = rx.recv() => {
                    match maybe_req {
                        Some(req) => pending.push(req),
                        None => {
                            flush_batch(&mut pending, &processor, &stats).await;
                            return;
                        }
                    }
                }
                _ = &mut sleep => {
                    stats.batch_timeouts_total.fetch_add(1, Ordering::Relaxed);
                    flush_batch(&mut pending, &processor, &stats).await;
                    first_enqueued_at = None;
                }
            }
        }
    }

    if !pending.is_empty() {
        flush_batch(&mut pending, &processor, &stats).await;
    }
}

async fn flush_batch<T, R>(
    pending: &mut Vec<BatchRequest<T, R>>,
    processor: &Arc<dyn BatchProcessor<T, R>>,
    stats: &Arc<BatcherStats>,
) where
    T: Send + 'static,
    R: Send + 'static,
{
    if pending.is_empty() {
        return;
    }

    let requests = std::mem::take(pending);

    let mut final_payloads = Vec::with_capacity(requests.len());
    let mut responders = Vec::with_capacity(requests.len());
    for req in requests {
        let wait_ms = req.enqueued_at.elapsed().as_millis() as u64;
        stats.queue_wait_ms_total.fetch_add(wait_ms, Ordering::Relaxed);
        stats.queue_wait_tracker.observe(wait_ms as f64);
        final_payloads.push(req.payload);
        responders.push(req.response_tx);
    }

    let started = Instant::now();
    let result = processor.process(final_payloads).await;
    let inference_ms = started.elapsed().as_millis() as u64;
    stats
        .inference_ms_total
        .fetch_add(inference_ms, Ordering::Relaxed);
    stats.inference_tracker.observe(inference_ms as f64);

    stats.batches_total.fetch_add(1, Ordering::Relaxed);
    stats
        .processed_items_total
        .fetch_add(responders.len() as u64, Ordering::Relaxed);
    decrease_queue_depth(stats, responders.len());

    match result {
        Ok(outputs) if outputs.len() == responders.len() => {
            for (output, responder) in outputs.into_iter().zip(responders.into_iter()) {
                if responder.send(Ok(output)).is_err() {
                    stats.responses_dropped_total.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        Ok(outputs) => {
            let err = anyhow::anyhow!(
                "INFERENCE_BATCH_FAILED: backend returned {} items for {} requests",
                outputs.len(),
                responders.len()
            );
            for responder in responders {
                if responder.send(Err(anyhow::anyhow!(err.to_string()))).is_err() {
                    stats.responses_dropped_total.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        Err(error) => {
            let message = error.to_string();
            for responder in responders {
                if responder
                    .send(Err(anyhow::anyhow!(
                        "INFERENCE_BATCH_FAILED: {}",
                        message
                    )))
                    .is_err()
                {
                    stats.responses_dropped_total.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
}

fn decrease_queue_depth(stats: &BatcherStats, delta: usize) {
    let _ = stats
        .queue_depth
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            Some(current.saturating_sub(delta))
        });
}
