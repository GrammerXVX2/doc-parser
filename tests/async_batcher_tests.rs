use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;

use document_parser::performance::{AsyncBatcher, BatchProcessor};

struct EchoProcessor {
    batch_sizes: Arc<Mutex<Vec<usize>>>,
    fail: bool,
}

#[async_trait]
impl BatchProcessor<u32, u32> for EchoProcessor {
    async fn process(&self, batch: Vec<u32>) -> anyhow::Result<Vec<u32>> {
        self.batch_sizes.lock().unwrap().push(batch.len());
        if self.fail {
            return Err(anyhow::anyhow!("backend failed"));
        }
        Ok(batch)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn flushes_by_max_batch_size() {
    let sizes = Arc::new(Mutex::new(Vec::new()));
    let processor = Arc::new(EchoProcessor {
        batch_sizes: sizes.clone(),
        fail: false,
    });

    let batcher = AsyncBatcher::new(2, Duration::from_millis(100), processor);

    let f1 = batcher.enqueue(10);
    let f2 = batcher.enqueue(20);

    let (r1, r2) = tokio::join!(f1, f2);
    assert_eq!(r1.unwrap(), 10);
    assert_eq!(r2.unwrap(), 20);

    tokio::time::sleep(Duration::from_millis(20)).await;
    let observed = sizes.lock().unwrap().clone();
    assert_eq!(observed, vec![2]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn flushes_by_timeout() {
    let sizes = Arc::new(Mutex::new(Vec::new()));
    let processor = Arc::new(EchoProcessor {
        batch_sizes: sizes.clone(),
        fail: false,
    });

    let batcher = AsyncBatcher::new(16, Duration::from_millis(30), processor);
    let output = batcher.enqueue(7).await.unwrap();
    assert_eq!(output, 7);

    tokio::time::sleep(Duration::from_millis(50)).await;
    let observed = sizes.lock().unwrap().clone();
    assert_eq!(observed, vec![1]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn routes_backend_errors_to_responses() {
    let sizes = Arc::new(Mutex::new(Vec::new()));
    let processor = Arc::new(EchoProcessor {
        batch_sizes: sizes,
        fail: true,
    });

    let batcher = AsyncBatcher::new(4, Duration::from_millis(20), processor);
    let err = batcher.enqueue(1).await.unwrap_err().to_string();
    assert!(err.contains("INFERENCE_BATCH_FAILED"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_rejects_new_requests() {
    let sizes = Arc::new(Mutex::new(Vec::new()));
    let processor = Arc::new(EchoProcessor {
        batch_sizes: sizes,
        fail: false,
    });

    let batcher = AsyncBatcher::new(4, Duration::from_millis(20), processor);
    batcher.close();
    tokio::time::sleep(Duration::from_millis(10)).await;

    let err = batcher.enqueue(1).await.unwrap_err().to_string();
    assert!(err.contains("BATCHER_SHUTDOWN"));
}
