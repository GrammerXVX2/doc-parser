pub mod async_batching;
pub mod backend;
pub mod crop;
pub mod decoder;
pub mod detection;
pub mod dynamic_batcher;
pub mod gpu_ocr;
pub mod metrics;
pub mod mock_ocr;
pub mod onnx_ocr;
pub mod postprocessing;
pub mod preprocessing;
pub mod recognition;
pub mod triton_ocr;
pub mod traits;
pub mod types;

pub use async_batching::{
	BatchedTextRecognizer, OcrCropRequest, SyncRecognizerBatchBackend, TextRecognizerBatchBackend,
};
pub use backend::{OcrBackendFactory, OcrBackendWarning};
pub use crop::{CropExtractor, OcrCrop};
pub use decoder::{CtcGreedyDecoder, DecodedText, RecognitionDecoder, ctc_greedy_decode_indices};
pub use dynamic_batcher::{DynamicBatcher, chunk_batches};
pub use gpu_ocr::GpuOcrSupport;
pub use mock_ocr::{MockOcrConfig, MockOcrPipeline};
pub use onnx_ocr::OnnxOcrPipeline;
pub use preprocessing::{PreprocessedImage, load_image_rgb, resize_with_padding};
pub use traits::{OcrPipeline, TextDetector, TextRecognizer};
pub use triton_ocr::{
	TritonOcrConfig, TritonOcrDetector, TritonOcrRecognizer, run_triton_page_ocr,
};
pub use types::{
	DetectionConfig, OcrBackendKind, OcrConfig, OcrFixtureLine, OcrPageInput,
	OcrPreprocessingConfig, OcrTritonConfig, RecognitionConfig, RecognizedText, TextRegion,
};
