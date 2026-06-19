use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use crate::ocr::crop::OcrCrop;
use crate::ocr::traits::TextRecognizer;
use crate::ocr::types::RecognizedText;
use crate::performance::{AsyncBatcher, BatchProcessor, BatcherSnapshot};

#[derive(Debug, Clone)]
pub struct OcrCropRequest {
    pub crop: OcrCrop,
}

#[async_trait]
pub trait TextRecognizerBatchBackend: Send + Sync {
    async fn recognize_batch_backend(
        &self,
        batch: Vec<OcrCropRequest>,
    ) -> anyhow::Result<Vec<RecognizedText>>;
}

struct RecognizerBatchProcessor {
    backend: Arc<dyn TextRecognizerBatchBackend>,
}

#[async_trait]
impl BatchProcessor<OcrCropRequest, RecognizedText> for RecognizerBatchProcessor {
    async fn process(&self, batch: Vec<OcrCropRequest>) -> anyhow::Result<Vec<RecognizedText>> {
        self.backend.recognize_batch_backend(batch).await
    }
}

pub struct BatchedTextRecognizer {
    inner: Arc<dyn TextRecognizerBatchBackend>,
    batcher: AsyncBatcher<OcrCropRequest, RecognizedText>,
}

impl BatchedTextRecognizer {
    pub fn new(
        inner: Arc<dyn TextRecognizerBatchBackend>,
        max_batch_size: usize,
        max_wait: Duration,
    ) -> Self {
        let processor = Arc::new(RecognizerBatchProcessor {
            backend: inner.clone(),
        });
        let batcher = AsyncBatcher::new(max_batch_size.max(1), max_wait, processor);
        Self { inner, batcher }
    }

    pub fn snapshot(&self) -> BatcherSnapshot {
        self.batcher.snapshot()
    }

    pub fn close(&self) {
        self.batcher.close();
    }

    pub fn backend(&self) -> Arc<dyn TextRecognizerBatchBackend> {
        self.inner.clone()
    }
}

impl TextRecognizer for BatchedTextRecognizer {
    fn recognize_batch(&self, crops: Vec<OcrCrop>) -> anyhow::Result<Vec<RecognizedText>> {
        if crops.is_empty() {
            return Ok(vec![]);
        }

        block_on_batched(async {
            let mut receivers = Vec::with_capacity(crops.len());
            for crop in crops {
                let receiver = self
                    .batcher
                    .submit(OcrCropRequest { crop })
                    .await?;
                receivers.push(receiver);
            }

            let mut outputs = Vec::with_capacity(receivers.len());
            for receiver in receivers {
                let item = receiver
                    .await
                    .map_err(|_| anyhow::anyhow!("BATCHER_RESPONSE_DROPPED: response channel closed"))??;
                outputs.push(item);
            }

            Ok(outputs)
        })
    }
}

pub struct SyncRecognizerBatchBackend {
    inner: Arc<dyn TextRecognizer + Send + Sync>,
}

impl SyncRecognizerBatchBackend {
    pub fn new(inner: Arc<dyn TextRecognizer + Send + Sync>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl TextRecognizerBatchBackend for SyncRecognizerBatchBackend {
    async fn recognize_batch_backend(
        &self,
        batch: Vec<OcrCropRequest>,
    ) -> anyhow::Result<Vec<RecognizedText>> {
        let crops = batch.into_iter().map(|item| item.crop).collect::<Vec<_>>();
        self.inner.recognize_batch(crops)
    }
}

fn block_on_batched<F, T>(future: F) -> anyhow::Result<T>
where
    F: std::future::Future<Output = anyhow::Result<T>>,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(future))
    } else {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(future)
    }
}
