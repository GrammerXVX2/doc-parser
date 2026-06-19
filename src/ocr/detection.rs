use std::path::Path;

use crate::ml::{OnnxSession, OnnxTensor};
use crate::ocr::postprocessing::{DetectionModelKind, DetectionPostProcessor, GenericBoxesPostProcessor};
use crate::ocr::preprocessing::{load_image_rgb, resize_with_padding};
use crate::ocr::traits::TextDetector;
use crate::ocr::types::{DetectionConfig, OcrPageInput, TextRegion};

pub struct OnnxTextDetector {
    session: OnnxSession,
    config: DetectionConfig,
    model_kind: DetectionModelKind,
    postprocessor: Box<dyn DetectionPostProcessor + Send + Sync>,
}

impl OnnxTextDetector {
    pub fn new(config: DetectionConfig) -> anyhow::Result<Self> {
        let session = OnnxSession::new(Path::new(&config.model_path), config.provider)?;
        Ok(Self {
            session,
            config,
            model_kind: DetectionModelKind::GenericBoxes,
            postprocessor: Box::new(GenericBoxesPostProcessor),
        })
    }
}

impl TextDetector for OnnxTextDetector {
    fn detect_page(&self, input: &OcrPageInput) -> anyhow::Result<Vec<TextRegion>> {
        let image = load_image_rgb(&input.image_path)?;
        let preprocessed = resize_with_padding(
            &image,
            self.config.input_width,
            self.config.input_height,
            image::Rgb([255, 255, 255]),
        )?;

        let _kind = self.model_kind;
        let outputs = self.session.run(vec![OnnxTensor::new(
            "images",
            vec![
                1,
                self.config.input_channels as usize,
                self.config.input_height as usize,
                self.config.input_width as usize,
            ],
            preprocessed.data_f32_chw.clone(),
        )])?;

        self.postprocessor.postprocess(outputs, &preprocessed)
    }
}
