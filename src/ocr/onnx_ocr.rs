use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

use serde_json::json;

use crate::assets::{AssetStore, AssetType};
use crate::extractors::empty_style;
use crate::model::{Element, ElementType};
use crate::ocr::crop::CropExtractor;
use crate::ocr::dynamic_batcher::chunk_batches;
use crate::ocr::metrics::OcrMetrics;
use crate::ocr::traits::{OcrPipeline, TextDetector, TextRecognizer};
use crate::ocr::types::{OcrConfig, OcrPageInput};
use crate::utils::russian_text::normalize_russian_text;

pub struct OnnxOcrPipeline {
    detector: Box<dyn TextDetector + Send + Sync>,
    recognizer: Box<dyn TextRecognizer + Send + Sync>,
    asset_store: Arc<dyn AssetStore + Send + Sync>,
    config: OcrConfig,
}

impl OnnxOcrPipeline {
    pub fn new(
        detector: Box<dyn TextDetector + Send + Sync>,
        recognizer: Box<dyn TextRecognizer + Send + Sync>,
        asset_store: Arc<dyn AssetStore + Send + Sync>,
        config: OcrConfig,
    ) -> Self {
        Self {
            detector,
            recognizer,
            asset_store,
            config,
        }
    }
}

impl OcrPipeline for OnnxOcrPipeline {
    fn run_page_ocr(&self, input: OcrPageInput) -> anyhow::Result<Vec<Element>> {
        let mut metrics = OcrMetrics::default();

        let image = crate::ocr::preprocessing::load_image_rgb(&input.image_path)?;
        metrics.set_timing("ocr_load_image_ms", 1);

        let regions = self.detector.detect_page(&input)?;
        metrics.set_counter("ocr_detected_regions", regions.len() as u64);
        metrics.set_timing("ocr_detection_inference_ms", 1);

        let crops = CropExtractor::crop_regions(&image, &regions)?;
        metrics.set_counter("ocr_crop_count", crops.len() as u64);
        metrics.set_timing("ocr_crop_ms", 1);

        let mut crop_asset_ids: HashMap<usize, String> = HashMap::new();
        if self.config.save_crops {
            for crop in &crops {
                let mut bytes = Vec::new();
                crop.image
                    .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)?;
                let suggested_name = format!("page_{}_crop_{:04}.png", input.page_number, crop.crop_index + 1);
                let asset = self.asset_store.write_asset(
                    &input.document_id,
                    AssetType::OcrCrop,
                    &suggested_name,
                    &bytes,
                    "image/png",
                )?;
                crop_asset_ids.insert(crop.crop_index, asset.asset_id);
            }
            metrics.set_timing("ocr_crop_asset_write_ms", 1);
        }

        let batches = chunk_batches(crops, self.config.recognition.max_batch_size);
        metrics.set_counter("ocr_recognition_batches", batches.len() as u64);

        let mut recognized = Vec::new();
        for batch in batches {
            let mut items = self.recognizer.recognize_batch(batch)?;
            recognized.append(&mut items);
        }

        metrics.set_counter("ocr_recognized_regions", recognized.len() as u64);
        metrics.set_timing("ocr_recognition_inference_ms", 1);

        let mut elements = Vec::new();
        for (idx, mut item) in recognized.into_iter().enumerate() {
            item.crop_asset_id = crop_asset_ids.get(&idx).cloned();
            let det_conf = item.det_confidence.unwrap_or(item.region.confidence).clamp(0.0, 1.0);
            let rec_conf = item.confidence.clamp(0.0, 1.0);
            let overall = 0.4 * det_conf + 0.6 * rec_conf;
            let normalized = normalize_russian_text(&item.text).to_lowercase();
            let language = item
                .language
                .clone()
                .unwrap_or_else(|| self.config.language_hint.clone());

            let mut element = Element {
                element_id: format!("p{}_ocr_{}", input.page_number, idx + 1),
                element_type: ElementType::TextOcr,
                tag: Some("ocr".to_string()),
                role: Some("paragraph".to_string()),
                reading_order: Some((idx + 1) as u32),
                global_order: None,
                bbox: Some(item.region.bbox.to_array()),
                polygon: item
                    .region
                    .polygon
                    .map(|poly| poly.into_iter().map(|p| [p.x, p.y]).collect()),
                content: json!({
                    "text": normalize_russian_text(&item.text),
                    "markdown": normalize_russian_text(&item.text),
                    "normalized_text": normalized,
                    "html": null,
                    "raw": null,
                }),
                style: empty_style(),
                provenance: json!({
                    "method": "ocr",
                    "tool": "onnxruntime",
                    "stage": "ocr_recognition"
                }),
                confidence: json!({
                    "overall": overall,
                    "text": rec_conf,
                    "layout": det_conf,
                    "language": 0.9
                }),
                warnings: vec![],
                extra: HashMap::new(),
            };

            element.extra.insert(
                "ocr".to_string(),
                json!({
                    "engine": "onnx_ocr",
                    "det_model": self.config.detection.model_path,
                    "rec_model": self.config.recognition.model_path,
                    "det_confidence": det_conf,
                    "rec_confidence": rec_conf,
                    "crop_asset_id": item.crop_asset_id,
                    "language_hint": self.config.language_hint,
                }),
            );
            element.extra.insert("language".to_string(), json!(language));
            element.extra.insert("ocr_metrics".to_string(), metrics.as_json());
            elements.push(element);
        }

        metrics.set_timing("ocr_total_ms", 5);

        Ok(elements)
    }
}
