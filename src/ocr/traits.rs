use crate::model::Element;

use super::crop::OcrCrop;
use super::types::{OcrPageInput, RecognizedText, TextRegion};

pub trait OcrPipeline {
    fn run_page_ocr(&self, input: OcrPageInput) -> anyhow::Result<Vec<Element>>;
}

pub trait TextDetector {
    fn detect_page(&self, input: &OcrPageInput) -> anyhow::Result<Vec<TextRegion>>;
}

pub trait TextRecognizer {
    fn recognize_batch(&self, crops: Vec<OcrCrop>) -> anyhow::Result<Vec<RecognizedText>>;
}
