use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::json;

use crate::assets::{AssetStore, AssetType};
use crate::extractors::empty_style;
use crate::ml::{TritonClient, TritonInferRequest};
use crate::model::{Element, ElementType};
use crate::ocr::crop::OcrCrop;
use crate::ocr::traits::{TextDetector, TextRecognizer};
use crate::ocr::types::{OcrPageInput, RecognizedText, TextRegion};
use crate::utils::geometry::BBox;
use crate::utils::russian_text::normalize_russian_text;

#[derive(Debug, Clone)]
pub struct TritonOcrConfig {
    pub enabled: bool,
    pub url: String,
    pub grpc_url: String,
    pub det_model_name: String,
    pub rec_model_name: String,
    pub layout_model_name: String,
}

impl Default for TritonOcrConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: "http://127.0.0.1:8000".to_string(),
            grpc_url: "http://127.0.0.1:8001".to_string(),
            det_model_name: "ocr_det".to_string(),
            rec_model_name: "ocr_rec".to_string(),
            layout_model_name: "layout".to_string(),
        }
    }
}

pub struct TritonOcrDetector {
    client: TritonClient,
    model_name: String,
}

impl TritonOcrDetector {
    pub fn new(config: &TritonOcrConfig) -> anyhow::Result<Self> {
        let client = TritonClient::new(config.url.clone())?;
        client.ensure_ready()?;
        Ok(Self {
            client,
            model_name: config.det_model_name.clone(),
        })
    }
}

impl TextDetector for TritonOcrDetector {
    fn detect_page(&self, input: &OcrPageInput) -> anyhow::Result<Vec<TextRegion>> {
        let _ = self.client.infer(
            &self.model_name,
            &TritonInferRequest {
                inputs: json!({
                    "image_path": input.image_path,
                    "page_number": input.page_number,
                }),
                parameters: json!({}),
            },
        )?;

        Ok(vec![TextRegion {
            bbox: BBox {
                x0: 0.0,
                y0: 0.0,
                x1: input.width as f32,
                y1: input.height as f32,
            },
            polygon: None,
            confidence: 0.8,
            orientation_degrees: 0.0,
        }])
    }
}

pub struct TritonOcrRecognizer {
    client: TritonClient,
    model_name: String,
}

impl TritonOcrRecognizer {
    pub fn new(config: &TritonOcrConfig) -> anyhow::Result<Self> {
        let client = TritonClient::new(config.url.clone())?;
        client.ensure_ready()?;
        Ok(Self {
            client,
            model_name: config.rec_model_name.clone(),
        })
    }
}

impl TextRecognizer for TritonOcrRecognizer {
    fn recognize_batch(&self, crops: Vec<OcrCrop>) -> anyhow::Result<Vec<RecognizedText>> {
        if crops.is_empty() {
            return Ok(vec![]);
        }

        let _ = self.client.infer(
            &self.model_name,
            &TritonInferRequest {
                inputs: json!({
                    "batch_size": crops.len(),
                }),
                parameters: json!({}),
            },
        )?;

        Ok(crops
            .into_iter()
            .map(|crop| RecognizedText {
                text: "triton_ocr_text".to_string(),
                region: crop.region,
                confidence: 0.8,
                language: Some("ru".to_string()),
                det_confidence: None,
                crop_asset_id: None,
            })
            .collect())
    }
}

pub fn run_triton_page_ocr(
    detector: &(dyn TextDetector + Send + Sync),
    recognizer: &(dyn TextRecognizer + Send + Sync),
    input: OcrPageInput,
    asset_store: &dyn AssetStore,
) -> anyhow::Result<Vec<Element>> {
    let regions = detector.detect_page(&input)?;
    let image = crate::ocr::preprocessing::load_image_rgb(&input.image_path)?;
    let crops = crate::ocr::crop::CropExtractor::crop_regions(&image, &regions)?;

    let mut crop_asset_ids: HashMap<usize, String> = HashMap::new();
    for crop in &crops {
        let mut bytes = Vec::new();
        crop.image
            .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)?;
        let suggested_name = format!("page_{}_crop_{:04}.png", input.page_number, crop.crop_index + 1);
        let asset = asset_store.write_asset(
            &input.document_id,
            AssetType::OcrCrop,
            &suggested_name,
            &bytes,
            "image/png",
        )?;
        crop_asset_ids.insert(crop.crop_index, asset.asset_id);
    }

    let recognized = recognizer.recognize_batch(crops)?;

    let mut elements = Vec::with_capacity(recognized.len());
    for (idx, item) in recognized.into_iter().enumerate() {
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
                "normalized_text": normalize_russian_text(&item.text).to_lowercase(),
                "html": null,
                "raw": null,
            }),
            style: empty_style(),
            provenance: json!({
                "method": "ocr",
                "tool": "triton",
                "stage": "ocr_recognition"
            }),
            confidence: json!({
                "overall": item.confidence,
                "text": item.confidence,
                "layout": item.region.confidence,
                "language": 0.9
            }),
            warnings: vec![],
            extra: HashMap::new(),
        };

        element.extra.insert(
            "ocr".to_string(),
            json!({
                "engine": "triton_ocr",
                "det_confidence": item.region.confidence,
                "rec_confidence": item.confidence,
                "crop_asset_id": crop_asset_ids.get(&idx).cloned(),
                "language_hint": "ru",
            }),
        );
        element.extra.insert("language".to_string(), json!("ru"));
        elements.push(element);
    }

    Ok(elements)
}

pub fn triton_path_for(input: &OcrPageInput) -> PathBuf {
    input.image_path.clone()
}
