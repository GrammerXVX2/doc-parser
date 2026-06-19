use std::sync::Arc;

use image::{DynamicImage, Rgb, RgbImage};

use document_parser::assets::LocalAssetStore;
use document_parser::ocr::onnx_ocr::OnnxOcrPipeline;
use document_parser::ocr::types::{OcrConfig, OcrPageInput, RecognizedText, TextRegion};
use document_parser::ocr::{OcrPipeline, TextDetector, TextRecognizer};
use document_parser::utils::geometry::BBox;

#[derive(Debug)]
struct FakeDetector;

impl TextDetector for FakeDetector {
    fn detect_page(&self, _input: &OcrPageInput) -> anyhow::Result<Vec<TextRegion>> {
        Ok(vec![TextRegion {
            bbox: BBox {
                x0: 10.0,
                y0: 10.0,
                x1: 120.0,
                y1: 40.0,
            },
            polygon: None,
            confidence: 0.95,
            orientation_degrees: 0.0,
        }])
    }
}

#[derive(Debug)]
struct FakeRecognizer;

impl TextRecognizer for FakeRecognizer {
    fn recognize_batch(
        &self,
        crops: Vec<document_parser::ocr::crop::OcrCrop>,
    ) -> anyhow::Result<Vec<RecognizedText>> {
        Ok(crops
            .into_iter()
            .map(|crop| RecognizedText {
                text: "Invoice number: 12345".to_string(),
                region: crop.region.clone(),
                confidence: 0.93,
                language: Some("en".to_string()),
                det_confidence: Some(crop.region.confidence),
                crop_asset_id: None,
            })
            .collect())
    }
}

fn temp_output_dir() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("doc_parser_stage3_{}", uuid::Uuid::new_v4()))
}

#[test]
fn onnx_pipeline_creates_ocr_elements_with_crop_asset_and_confidence() {
    let output_dir = temp_output_dir();
    let image_path = output_dir.join("scan.png");
    std::fs::create_dir_all(&output_dir).expect("temp dir");

    let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(300, 120, Rgb([255, 255, 255])));
    image.save(&image_path).expect("save image");

    let mut cfg = OcrConfig::default();
    cfg.save_crops = true;

    let store: Arc<dyn document_parser::assets::AssetStore + Send + Sync> =
        Arc::new(LocalAssetStore::new(&output_dir));

    let pipeline = OnnxOcrPipeline::new(
        Box::new(FakeDetector),
        Box::new(FakeRecognizer),
        store,
        cfg,
    );

    let elements = pipeline
        .run_page_ocr(OcrPageInput {
            document_id: "doc_test".to_string(),
            page_number: 1,
            image_asset_id: "asset_img_1".to_string(),
            image_path,
            width: 300,
            height: 120,
            dpi: None,
        })
        .expect("ocr run should succeed");

    assert_eq!(elements.len(), 1);
    let el = &elements[0];
    assert_eq!(el.element_type, document_parser::model::ElementType::TextOcr);
    assert_eq!(el.provenance["method"].as_str(), Some("ocr"));
    assert!(el.content["text"].as_str().unwrap_or("").contains("Invoice"));

    let overall = el.confidence["overall"].as_f64().unwrap_or(0.0);
    assert!(overall > 0.0);

    let crop_asset = el
        .extra
        .get("ocr")
        .and_then(|v| v["crop_asset_id"].as_str());
    assert!(crop_asset.is_some());
}
