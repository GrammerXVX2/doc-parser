use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ml::ExecutionProviderKind;
use crate::utils::geometry::{BBox, Point};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OcrBackendKind {
    Disabled,
    Mock,
    Onnx,
    Triton,
}

impl OcrBackendKind {
    pub fn from_str(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "disabled" => Self::Disabled,
            "onnx" => Self::Onnx,
            "triton" => Self::Triton,
            _ => Self::Mock,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrTritonConfig {
    pub enabled: bool,
    pub url: String,
    pub grpc_url: String,
    pub det_model_name: String,
    pub rec_model_name: String,
    pub layout_model_name: String,
}

impl Default for OcrTritonConfig {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrPreprocessingConfig {
    pub apply_exif_orientation: bool,
    pub convert_to_rgb: bool,
    pub normalize: bool,
    pub deskew: bool,
    pub denoise: bool,
    pub preserve_aspect_ratio: bool,
    pub padding_color: String,
}

impl Default for OcrPreprocessingConfig {
    fn default() -> Self {
        Self {
            apply_exif_orientation: true,
            convert_to_rgb: true,
            normalize: true,
            deskew: false,
            denoise: false,
            preserve_aspect_ratio: true,
            padding_color: "white".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionConfig {
    pub model_path: String,
    pub provider: ExecutionProviderKind,
    pub input_width: u32,
    pub input_height: u32,
    pub input_channels: u32,
    pub max_batch_size: usize,
    pub max_wait_ms: u64,
    pub confidence_threshold: f32,
    pub box_threshold: f32,
    pub unclip_ratio: f32,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            model_path: "models/ocr/det.onnx".to_string(),
            provider: ExecutionProviderKind::Cpu,
            input_width: 1024,
            input_height: 1024,
            input_channels: 3,
            max_batch_size: 8,
            max_wait_ms: 15,
            confidence_threshold: 0.5,
            box_threshold: 0.5,
            unclip_ratio: 1.6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognitionConfig {
    pub model_path: String,
    pub provider: ExecutionProviderKind,
    pub input_width: u32,
    pub input_height: u32,
    pub input_channels: u32,
    pub max_batch_size: usize,
    pub max_wait_ms: u64,
    pub confidence_threshold: f32,
    pub charset_path: String,
    pub decoder: String,
}

impl Default for RecognitionConfig {
    fn default() -> Self {
        Self {
            model_path: "models/ocr/rec.onnx".to_string(),
            provider: ExecutionProviderKind::Cpu,
            input_width: 320,
            input_height: 48,
            input_channels: 3,
            max_batch_size: 64,
            max_wait_ms: 15,
            confidence_threshold: 0.5,
            charset_path: "models/ocr/charset.txt".to_string(),
            decoder: "ctc".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    pub enabled: bool,
    pub backend: OcrBackendKind,
    pub fallback_to_mock: bool,
    pub languages: Vec<String>,
    pub language_hint: String,
    pub locale: String,
    pub save_crops: bool,
    pub save_debug_artifacts: bool,
    pub preprocessing: OcrPreprocessingConfig,
    pub detection: DetectionConfig,
    pub recognition: RecognitionConfig,
    pub mock_fixture_based: bool,
    pub mock_deterministic_fallback: bool,
    pub recognition_batching_enabled: bool,
    pub recognition_batching_max_batch_size: usize,
    pub recognition_batching_max_wait_ms: u64,
    pub triton: OcrTritonConfig,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: OcrBackendKind::Mock,
            fallback_to_mock: true,
            languages: vec!["ru".to_string(), "en".to_string()],
            language_hint: "ru".to_string(),
            locale: "ru".to_string(),
            save_crops: true,
            save_debug_artifacts: false,
            preprocessing: OcrPreprocessingConfig::default(),
            detection: DetectionConfig::default(),
            recognition: RecognitionConfig::default(),
            mock_fixture_based: true,
            mock_deterministic_fallback: true,
            recognition_batching_enabled: true,
            recognition_batching_max_batch_size: 128,
            recognition_batching_max_wait_ms: 15,
            triton: OcrTritonConfig::default(),
        }
    }
}

impl OcrConfig {
    pub fn from_pipeline_ocr_value(ocr_value: &Value) -> Self {
        let mut config = Self::default();

        config.enabled = ocr_value.get("enabled").and_then(Value::as_bool).unwrap_or(true);
        config.backend = OcrBackendKind::from_str(
            ocr_value
                .get("backend")
                .and_then(Value::as_str)
                .unwrap_or("mock"),
        );
        config.fallback_to_mock = ocr_value
            .get("fallback_to_mock")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        config.languages = ocr_value
            .get("languages")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .filter(|langs| !langs.is_empty())
            .unwrap_or_else(|| vec!["ru".to_string(), "en".to_string()]);
        config.language_hint = ocr_value
            .get("language_hint")
            .and_then(Value::as_str)
            .unwrap_or("ru")
            .to_string();
        config.locale = ocr_value
            .get("locale")
            .and_then(Value::as_str)
            .unwrap_or("ru")
            .to_string();
        config.save_crops = ocr_value
            .get("save_crops")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        config.save_debug_artifacts = ocr_value
            .get("save_debug_artifacts")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        if let Some(pre) = ocr_value.get("preprocessing") {
            config.preprocessing.apply_exif_orientation = pre
                .get("apply_exif_orientation")
                .and_then(Value::as_bool)
                .unwrap_or(config.preprocessing.apply_exif_orientation);
            config.preprocessing.convert_to_rgb = pre
                .get("convert_to_rgb")
                .and_then(Value::as_bool)
                .unwrap_or(config.preprocessing.convert_to_rgb);
            config.preprocessing.normalize = pre
                .get("normalize")
                .and_then(Value::as_bool)
                .unwrap_or(config.preprocessing.normalize);
            config.preprocessing.deskew = pre
                .get("deskew")
                .and_then(Value::as_bool)
                .unwrap_or(config.preprocessing.deskew);
            config.preprocessing.denoise = pre
                .get("denoise")
                .and_then(Value::as_bool)
                .unwrap_or(config.preprocessing.denoise);
            config.preprocessing.preserve_aspect_ratio = pre
                .get("preserve_aspect_ratio")
                .and_then(Value::as_bool)
                .unwrap_or(config.preprocessing.preserve_aspect_ratio);
            config.preprocessing.padding_color = pre
                .get("padding_color")
                .and_then(Value::as_str)
                .unwrap_or("white")
                .to_string();
        }

        if let Some(det) = ocr_value.get("detection") {
            config.detection.model_path = det
                .get("model_path")
                .and_then(Value::as_str)
                .unwrap_or(&config.detection.model_path)
                .to_string();
            config.detection.provider = ExecutionProviderKind::from_str(
                det.get("provider")
                    .and_then(Value::as_str)
                    .unwrap_or("cpu"),
            );
            config.detection.input_width = det
                .get("input_width")
                .and_then(Value::as_u64)
                .map(|v| v as u32)
                .unwrap_or(config.detection.input_width);
            config.detection.input_height = det
                .get("input_height")
                .and_then(Value::as_u64)
                .map(|v| v as u32)
                .unwrap_or(config.detection.input_height);
            config.detection.input_channels = det
                .get("input_channels")
                .and_then(Value::as_u64)
                .map(|v| v as u32)
                .unwrap_or(config.detection.input_channels);
            config.detection.max_batch_size = det
                .get("max_batch_size")
                .and_then(Value::as_u64)
                .map(|v| v as usize)
                .unwrap_or(config.detection.max_batch_size);
            config.detection.max_wait_ms = det
                .get("max_wait_ms")
                .and_then(Value::as_u64)
                .unwrap_or(config.detection.max_wait_ms);
            config.detection.confidence_threshold = det
                .get("confidence_threshold")
                .and_then(Value::as_f64)
                .map(|v| v as f32)
                .unwrap_or(config.detection.confidence_threshold);
            config.detection.box_threshold = det
                .get("box_threshold")
                .and_then(Value::as_f64)
                .map(|v| v as f32)
                .unwrap_or(config.detection.box_threshold);
            config.detection.unclip_ratio = det
                .get("unclip_ratio")
                .and_then(Value::as_f64)
                .map(|v| v as f32)
                .unwrap_or(config.detection.unclip_ratio);
        }

        if let Some(rec) = ocr_value.get("recognition") {
            config.recognition.model_path = rec
                .get("model_path")
                .and_then(Value::as_str)
                .unwrap_or(&config.recognition.model_path)
                .to_string();
            config.recognition.provider = ExecutionProviderKind::from_str(
                rec.get("provider")
                    .and_then(Value::as_str)
                    .unwrap_or("cpu"),
            );
            config.recognition.input_width = rec
                .get("input_width")
                .and_then(Value::as_u64)
                .map(|v| v as u32)
                .unwrap_or(config.recognition.input_width);
            config.recognition.input_height = rec
                .get("input_height")
                .and_then(Value::as_u64)
                .map(|v| v as u32)
                .unwrap_or(config.recognition.input_height);
            config.recognition.input_channels = rec
                .get("input_channels")
                .and_then(Value::as_u64)
                .map(|v| v as u32)
                .unwrap_or(config.recognition.input_channels);
            config.recognition.max_batch_size = rec
                .get("max_batch_size")
                .and_then(Value::as_u64)
                .map(|v| v as usize)
                .unwrap_or(config.recognition.max_batch_size);
            config.recognition.max_wait_ms = rec
                .get("max_wait_ms")
                .and_then(Value::as_u64)
                .unwrap_or(config.recognition.max_wait_ms);
            config.recognition.confidence_threshold = rec
                .get("confidence_threshold")
                .and_then(Value::as_f64)
                .map(|v| v as f32)
                .unwrap_or(config.recognition.confidence_threshold);
            config.recognition.charset_path = rec
                .get("charset_path")
                .and_then(Value::as_str)
                .unwrap_or(&config.recognition.charset_path)
                .to_string();
            config.recognition.decoder = rec
                .get("decoder")
                .and_then(Value::as_str)
                .unwrap_or(&config.recognition.decoder)
                .to_string();
        }

        if let Some(mock) = ocr_value.get("mock") {
            config.mock_fixture_based = mock
                .get("fixture_based")
                .and_then(Value::as_bool)
                .unwrap_or(config.mock_fixture_based);
            config.mock_deterministic_fallback = mock
                .get("deterministic_fallback")
                .and_then(Value::as_bool)
                .unwrap_or(config.mock_deterministic_fallback);
        }

        if let Some(batching) = ocr_value.get("batching") {
            if let Some(recognition) = batching.get("recognition") {
                config.recognition_batching_enabled = recognition
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(config.recognition_batching_enabled);
                config.recognition_batching_max_batch_size = recognition
                    .get("max_batch_size")
                    .and_then(Value::as_u64)
                    .map(|v| v as usize)
                    .unwrap_or(config.recognition_batching_max_batch_size);
                config.recognition_batching_max_wait_ms = recognition
                    .get("max_wait_ms")
                    .and_then(Value::as_u64)
                    .unwrap_or(config.recognition_batching_max_wait_ms);
            }
        }

        if let Some(triton) = ocr_value.get("triton") {
            config.triton.enabled = triton
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(config.triton.enabled);
            config.triton.url = triton
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or(&config.triton.url)
                .to_string();
            config.triton.grpc_url = triton
                .get("grpc_url")
                .and_then(Value::as_str)
                .unwrap_or(&config.triton.grpc_url)
                .to_string();
            config.triton.det_model_name = triton
                .get("det_model_name")
                .and_then(Value::as_str)
                .unwrap_or(&config.triton.det_model_name)
                .to_string();
            config.triton.rec_model_name = triton
                .get("rec_model_name")
                .and_then(Value::as_str)
                .unwrap_or(&config.triton.rec_model_name)
                .to_string();
            config.triton.layout_model_name = triton
                .get("layout_model_name")
                .and_then(Value::as_str)
                .unwrap_or(&config.triton.layout_model_name)
                .to_string();
        }

        config
    }

    pub fn apply_performance_overrides(&mut self, performance_value: &Value, ml_value: Option<&Value>) {
        if let Some(batching) = performance_value.get("batching") {
            if let Some(recognition) = batching.get("recognition") {
                self.recognition_batching_enabled = recognition
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(self.recognition_batching_enabled);
                self.recognition_batching_max_batch_size = recognition
                    .get("max_batch_size")
                    .and_then(Value::as_u64)
                    .map(|v| v as usize)
                    .unwrap_or(self.recognition_batching_max_batch_size);
                self.recognition_batching_max_wait_ms = recognition
                    .get("max_wait_ms")
                    .and_then(Value::as_u64)
                    .unwrap_or(self.recognition_batching_max_wait_ms);
            }
        }

        if let Some(ml) = ml_value {
            if let Some(provider) = ml.get("provider").and_then(Value::as_str) {
                let provider = ExecutionProviderKind::from_str(provider);
                self.detection.provider = provider;
                self.recognition.provider = provider;
                if matches!(provider, ExecutionProviderKind::Triton) {
                    self.backend = OcrBackendKind::Triton;
                }
            }

            if let Some(triton) = ml.get("triton") {
                self.triton.enabled = triton
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(self.triton.enabled);
                self.triton.url = triton
                    .get("url")
                    .and_then(Value::as_str)
                    .unwrap_or(&self.triton.url)
                    .to_string();
                self.triton.grpc_url = triton
                    .get("grpc_url")
                    .and_then(Value::as_str)
                    .unwrap_or(&self.triton.grpc_url)
                    .to_string();
                self.triton.det_model_name = triton
                    .get("det_model_name")
                    .and_then(Value::as_str)
                    .unwrap_or(&self.triton.det_model_name)
                    .to_string();
                self.triton.rec_model_name = triton
                    .get("rec_model_name")
                    .and_then(Value::as_str)
                    .unwrap_or(&self.triton.rec_model_name)
                    .to_string();
                self.triton.layout_model_name = triton
                    .get("layout_model_name")
                    .and_then(Value::as_str)
                    .unwrap_or(&self.triton.layout_model_name)
                    .to_string();
            }
        }
    }

    pub fn apply_cli_overrides(
        &mut self,
        backend: Option<&str>,
        det_model: Option<&str>,
        rec_model: Option<&str>,
        charset: Option<&str>,
        provider: Option<&str>,
        triton_url: Option<&str>,
        save_crops: Option<bool>,
    ) {
        if let Some(backend) = backend {
            self.backend = OcrBackendKind::from_str(backend);
            if matches!(self.backend, OcrBackendKind::Disabled) {
                self.enabled = false;
            }
        }
        if let Some(path) = det_model {
            self.detection.model_path = path.to_string();
        }
        if let Some(path) = rec_model {
            self.recognition.model_path = path.to_string();
        }
        if let Some(path) = charset {
            self.recognition.charset_path = path.to_string();
        }
        if let Some(provider) = provider {
            let provider = ExecutionProviderKind::from_str(provider);
            self.detection.provider = provider;
            self.recognition.provider = provider;
            if matches!(provider, ExecutionProviderKind::Triton) {
                self.backend = OcrBackendKind::Triton;
                self.triton.enabled = true;
            }
        }
        if let Some(url) = triton_url {
            self.triton.url = url.to_string();
            self.triton.enabled = true;
        }
        if let Some(save_crops) = save_crops {
            self.save_crops = save_crops;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRegion {
    pub bbox: BBox,
    pub polygon: Option<Vec<Point>>,
    pub confidence: f32,
    pub orientation_degrees: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognizedText {
    pub text: String,
    pub region: TextRegion,
    pub confidence: f32,
    pub language: Option<String>,
    pub det_confidence: Option<f32>,
    pub crop_asset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrPageInput {
    pub document_id: String,
    pub page_number: usize,
    pub image_asset_id: String,
    pub image_path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub dpi: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrFixtureLine {
    pub text: String,
    pub bbox: [f32; 4],
    pub confidence: f32,
}
