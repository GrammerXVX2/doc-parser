use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use serde_json::json;

use crate::assets::{AssetStore, AssetType, LocalAssetStore};
use crate::classifier::FileClassification;
use crate::config::load_pipeline_config;
use crate::extractors::{
    base_document_model, default_confidence, empty_style, provenance, stage, update_stats,
};
use crate::model::{Asset, ContentMode, Diagnostic, DocumentFormat, Element, ElementType, PageType};
use crate::ocr::{OcrBackendFactory, OcrBackendKind, OcrConfig, OcrPageInput};
use crate::runtime::{ocr_cli_overrides, output_root_dir};
use crate::router::Extractor;
use crate::utils::image_io::{image_dimensions, read_image_bytes};

#[derive(Default)]
pub struct ImageExtractor;

impl Extractor for ImageExtractor {
    fn name(&self) -> &'static str {
        "image_extractor"
    }

    fn extract(
        &self,
        input_path: &Path,
        classification: &FileClassification,
    ) -> anyhow::Result<crate::model::DocumentModel> {
        let bytes = read_image_bytes(input_path)?;
        let (width, height) = image_dimensions(&bytes)?;

        let mut model = base_document_model(
            classification,
            DocumentFormat::Image,
            ContentMode::Image,
            PageType::Image,
        );
        model.coordinate_system.unit = "px".to_string();
        model.document_profile.has_images = true;
        model.document_profile.has_native_text = false;
        model.document_profile.has_ocr_required_regions = true;

        let store = LocalAssetStore::new(output_root_dir());
        let image_name = input_path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("image.bin");

        let asset = store.write_asset(
            &model.document_id,
            AssetType::EmbeddedImage,
            image_name,
            &bytes,
            classification
                .mime_by_magic
                .as_deref()
                .or(classification.mime_by_extension.as_deref())
                .unwrap_or("application/octet-stream"),
        )?;

        model.assets.push(Asset {
            width: Some(width),
            height: Some(height),
            ..asset
        });
        let page_image_asset_id = model.assets[0].asset_id.clone();

        let mut page_text = String::new();
        let mut elements = Vec::new();

        model.processing.stages.push(stage("image_decode", "image", 1));

        elements.push(Element {
            element_id: "p1_e1".to_string(),
            element_type: ElementType::Image,
            tag: Some("img".to_string()),
            role: Some("page_image".to_string()),
            reading_order: Some(1),
            global_order: Some(1),
            bbox: Some([0.0, 0.0, width as f32, height as f32]),
            polygon: None,
            content: json!({
                "text": "",
                "html": null,
                "markdown": format!("![image]({})", model.assets[0].path),
                "normalized_text": "",
                "raw": null,
            }),
            style: empty_style(),
            provenance: provenance("image_decoder", "image_native", "path", "source_image"),
            confidence: default_confidence(),
            warnings: vec![],
            extra: {
                let mut map = HashMap::new();
                map.insert("asset_id".to_string(), json!(model.assets[0].asset_id.clone()));
                map
            },
        });

        let pipeline_cfg = load_pipeline_config(std::path::Path::new("configs/pipeline.config.jsonc")).ok();
        let mut ocr_config = pipeline_cfg
            .as_ref()
            .map(|cfg| OcrConfig::from_pipeline_ocr_value(&cfg.pipeline.ocr))
            .unwrap_or_default();
        if let Some(cfg) = &pipeline_cfg {
            ocr_config.apply_performance_overrides(&cfg.pipeline.performance, Some(&cfg.pipeline.ml));
        }

        if let Some(overrides) = ocr_cli_overrides() {
            ocr_config.apply_cli_overrides(
                overrides.backend.as_deref(),
                overrides.det_model.as_deref(),
                overrides.rec_model.as_deref(),
                overrides.charset.as_deref(),
                overrides.provider.as_deref(),
                overrides.triton_url.as_deref(),
                overrides.save_crops,
            );
        }

        if !ocr_config.enabled || matches!(ocr_config.backend, OcrBackendKind::Disabled) {
            model.warnings.push(Diagnostic {
                code: "OCR_DISABLED".to_string(),
                severity: "warning".to_string(),
                scope: "document".to_string(),
                page_number: None,
                element_id: None,
                message: "OCR disabled in config; image text extraction skipped".to_string(),
                recoverable: true,
                extra: HashMap::new(),
            });
        } else {
            let store: Arc<dyn AssetStore + Send + Sync> = Arc::new(LocalAssetStore::new(output_root_dir()));
            let (ocr_pipeline, backend_warnings) = OcrBackendFactory::create(&ocr_config, store)?;
            let effective_backend = if backend_warnings
                .iter()
                .any(|w| w.code == "OCR_BACKEND_FALLBACK_TO_MOCK")
            {
                OcrBackendKind::Mock
            } else {
                ocr_config.backend
            };
            for backend_warning in backend_warnings {
                model.warnings.push(Diagnostic {
                    code: backend_warning.code,
                    severity: "warning".to_string(),
                    scope: "stage".to_string(),
                    page_number: Some(1),
                    element_id: None,
                    message: backend_warning.message,
                    recoverable: true,
                    extra: HashMap::new(),
                });
            }

            let input = OcrPageInput {
                document_id: model.document_id.clone(),
                page_number: 1,
                image_asset_id: page_image_asset_id.clone(),
                image_path: input_path.to_path_buf(),
                width,
                height,
                dpi: None,
            };

            if matches!(effective_backend, OcrBackendKind::Onnx) {
                model.processing.stages.push(stage("ocr_load_image", "onnxruntime", 1));
                model.processing.stages.push(stage("ocr_detection", "onnxruntime", 1));
                model.processing.stages.push(stage("ocr_crop", "onnxruntime", 1));
            }

            let ocr_elements = ocr_pipeline.run_page_ocr(input)?;
            if !ocr_elements.is_empty() {
                page_text = ocr_elements
                    .iter()
                    .filter_map(|e| e.content.get("text").and_then(|v| v.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n");
                elements.extend(ocr_elements);
            }
            let tool = match effective_backend {
                OcrBackendKind::Onnx => "onnxruntime",
                OcrBackendKind::Triton => "triton",
                OcrBackendKind::Mock => "mock_ocr",
                OcrBackendKind::Disabled => "ocr_disabled",
            };
            model.processing.stages.push(stage("ocr_recognition", tool, 1));
            if matches!(effective_backend, OcrBackendKind::Onnx) {
                model.processing.stages.push(stage("ocr_total", "onnxruntime", 1));
            }
        }

        let page = &mut model.pages[0];
        page.width = Some(width as f32);
        page.height = Some(height as f32);
        page.page_image_asset_id = Some(page_image_asset_id);
        page.page_profile.content_mode = ContentMode::Image;
        page.page_profile.has_native_text = false;
        page.page_profile.has_images = true;
        page.page_profile.has_ocr_required_regions = model.document_profile.has_ocr_required_regions;
        page.elements = elements;
        page.text = page_text;
        page.markdown = page
            .elements
            .iter()
            .filter_map(|e| e.content.get("markdown").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join("\n\n");
        page.html = String::new();

        update_stats(&mut model);
        model.processing.total_duration_ms = Some(2);

        Ok(model)
    }
}
