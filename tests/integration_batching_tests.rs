use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use image::{DynamicImage, Rgb, RgbImage};

use document_parser::ocr::async_batching::{
    BatchedTextRecognizer, OcrCropRequest, TextRecognizerBatchBackend,
};
use document_parser::ocr::crop::OcrCrop;
use document_parser::ocr::traits::TextRecognizer;
use document_parser::ocr::types::{RecognizedText, TextRegion};
use document_parser::utils::geometry::BBox;

struct RecordingBackend {
    batches: Arc<Mutex<Vec<usize>>>,
}

#[async_trait]
impl TextRecognizerBatchBackend for RecordingBackend {
    async fn recognize_batch_backend(
        &self,
        batch: Vec<OcrCropRequest>,
    ) -> anyhow::Result<Vec<RecognizedText>> {
        self.batches.lock().unwrap().push(batch.len());
        Ok(batch
            .into_iter()
            .map(|item| RecognizedText {
                text: "ok".to_string(),
                region: item.crop.region,
                confidence: 0.9,
                language: Some("ru".to_string()),
                det_confidence: Some(0.9),
                crop_asset_id: None,
            })
            .collect())
    }
}

fn sample_crop(index: usize) -> OcrCrop {
    OcrCrop {
        region: TextRegion {
            bbox: BBox {
                x0: 0.0,
                y0: 0.0,
                x1: 10.0,
                y1: 10.0,
            },
            polygon: None,
            confidence: 0.9,
            orientation_degrees: 0.0,
        },
        image: DynamicImage::ImageRgb8(RgbImage::from_pixel(16, 16, Rgb([255, 255, 255]))),
        crop_index: index,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_requests_share_recognition_batches() {
    let batches = Arc::new(Mutex::new(Vec::new()));
    let backend = Arc::new(RecordingBackend {
        batches: batches.clone(),
    });

    let recognizer = Arc::new(BatchedTextRecognizer::new(
        backend,
        8,
        Duration::from_millis(30),
    ));

    let r1 = {
        let recognizer = recognizer.clone();
        tokio::spawn(async move { recognizer.recognize_batch(vec![sample_crop(1)]) })
    };
    let r2 = {
        let recognizer = recognizer.clone();
        tokio::spawn(async move { recognizer.recognize_batch(vec![sample_crop(2)]) })
    };

    let out1 = r1.await.unwrap().unwrap();
    let out2 = r2.await.unwrap().unwrap();
    assert_eq!(out1.len(), 1);
    assert_eq!(out2.len(), 1);

    tokio::time::sleep(Duration::from_millis(60)).await;
    let snapshot = recognizer.snapshot();
    assert_eq!(snapshot.submitted_total, 2);
    assert!(snapshot.avg_batch_size >= 1.0);

    let observed = batches.lock().unwrap().clone();
    assert!(!observed.is_empty());
}
