pub mod doc;
pub mod docx;
pub mod html;
pub mod image;
pub mod markdown;
pub mod pdf;
pub mod pptx;
pub mod rtf;
pub mod txt;
pub mod xlsx;

use std::collections::HashMap;

use chrono::Utc;
use serde_json::{Value, json};

use crate::classifier::FileClassification;
use crate::language::LanguageInfo;
use crate::model::{
    ContentMode, CoordinateSystem, Diagnostic, DocumentModel, DocumentProfile, DocumentStats,
    Hashes, Page, PageProfile, PageType, ProcessingStage, ProcessingStatus, ProcessingTrace,
    RuntimeInfo, SourceContainer, SourceInfo, StageStatus,
};

pub fn base_document_model(
    classification: &FileClassification,
    format: crate::model::DocumentFormat,
    content_mode: ContentMode,
    page_type: PageType,
) -> DocumentModel {
    let filename = classification
        .input_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    DocumentModel {
        schema_version: "1.0.0".to_string(),
        document_id: filename.replace('.', "_"),
        job_id: None,
        source: SourceInfo {
            uri: classification.input_path.to_string_lossy().to_string(),
            filename: filename.to_string(),
            extension: classification.extension.clone(),
            mime_type: classification
                .mime_by_magic
                .clone()
                .or_else(|| classification.mime_by_extension.clone())
                .unwrap_or_else(|| "application/octet-stream".to_string()),
            size_bytes: Some(classification.size_bytes),
            hashes: Hashes {
                sha256: Some(classification.sha256.clone()),
            },
            uploaded_at: None,
            processed_at: Some(Utc::now()),
            container: SourceContainer {
                container_type: None,
                parent_uri: None,
                entry_path: None,
            },
        },
        document_profile: DocumentProfile {
            format,
            content_mode: content_mode.clone(),
            languages: vec!["ru".to_string(), "en".to_string()],
            language_info: LanguageInfo::default(),
            has_native_text: true,
            has_images: false,
            has_tables: false,
            has_formulas: false,
            has_ocr_required_regions: false,
            has_handwriting: false,
            has_multicolumn_layout: false,
            document_type_guess: None,
            confidence: 0.9,
        },
        stats: DocumentStats::default(),
        coordinate_system: CoordinateSystem {
            origin: "top_left".to_string(),
            unit: if matches!(page_type, PageType::SyntheticTextPage) {
                "synthetic".to_string()
            } else {
                "px".to_string()
            },
            dpi: None,
            normalized_to_page: true,
        },
        assets: vec![],
        pages: vec![Page {
            page_number: 1,
            page_type,
            width: None,
            height: None,
            dpi: None,
            rotation_degrees: 0.0,
            page_image_asset_id: None,
            page_profile: PageProfile {
                content_mode,
                has_native_text: true,
                has_ocr_required_regions: false,
                has_tables: false,
                has_images: false,
                has_formulas: false,
                has_handwriting: false,
                language: Some("ru".to_string()),
                language_info: LanguageInfo::default(),
                confidence: 0.9,
            },
            elements: vec![],
            text: String::new(),
            markdown: String::new(),
            html: String::new(),
            warnings: vec![],
            extra: HashMap::new(),
        }],
        chunks: vec![],
        errors: vec![],
        warnings: vec![],
        processing: ProcessingTrace {
            pipeline_version: "1.0.0".to_string(),
            status: ProcessingStatus::Ok,
            stages: vec![],
            total_duration_ms: None,
            runtime: RuntimeInfo {
                hostname: None,
                cpu: None,
                gpu: None,
                cuda_version: None,
                onnxruntime_version: None,
            },
        },
        extra: HashMap::new(),
    }
}

pub fn provenance(tool: &str, stage: &str, source_kind: &str, source_value: &str) -> Value {
    json!({
        "method": "native",
        "tool": tool,
        "stage": stage,
        "source_ref": {
            "kind": source_kind,
            "value": source_value
        }
    })
}

pub fn default_confidence() -> Value {
    json!({
        "overall": 0.99,
        "text": 0.99,
        "layout": 0.9,
        "language": 0.9
    })
}

pub fn empty_style() -> Value {
    json!({
        "font_size": null,
        "font_family": null,
        "bold": false,
        "italic": false,
        "monospace": false,
        "color": null,
        "background_color": null,
        "alignment": "left"
    })
}

pub fn update_stats(model: &mut DocumentModel) {
    let mut element_count = 0_u32;
    let mut text_element_count = 0_u32;
    let mut ocr_element_count = 0_u32;
    let mut image_count = 0_u32;
    let mut table_count = 0_u32;
    let mut formula_count = 0_u32;
    let mut list_count = 0_u32;
    let mut total_chars = 0_u32;
    let mut total_words = 0_u32;

    for page in &model.pages {
        total_chars += page.text.chars().count() as u32;
        total_words += page.text.split_whitespace().count() as u32;
        for el in &page.elements {
            element_count += 1;
            match el.element_type {
                crate::model::ElementType::Text
                | crate::model::ElementType::TextOcr
                | crate::model::ElementType::Paragraph
                | crate::model::ElementType::Heading
                | crate::model::ElementType::Blockquote
                | crate::model::ElementType::Code
                | crate::model::ElementType::List
                | crate::model::ElementType::ListItem => {
                    text_element_count += 1;
                }
                crate::model::ElementType::Image => image_count += 1,
                crate::model::ElementType::Table => table_count += 1,
                crate::model::ElementType::Formula => formula_count += 1,
                _ => {}
            }
            if matches!(
                el.element_type,
                crate::model::ElementType::List | crate::model::ElementType::ListItem
            ) {
                list_count += 1;
            }
            if matches!(el.element_type, crate::model::ElementType::TextOcr) {
                ocr_element_count += 1;
            }
        }
    }

    model.stats.page_count = model.pages.len() as u32;
    model.stats.element_count = element_count;
    model.stats.text_element_count = text_element_count;
    model.stats.ocr_element_count = ocr_element_count;
    model.stats.image_count = image_count;
    model.stats.table_count = table_count;
    model.stats.formula_count = formula_count;
    model.stats.list_count = list_count;
    model.stats.total_chars = total_chars;
    model.stats.total_words = total_words;
}

pub fn stage(name: &str, tool: &str, duration_ms: u64) -> ProcessingStage {
    ProcessingStage {
        name: name.to_string(),
        status: StageStatus::Ok,
        tool: tool.to_string(),
        duration_ms: Some(duration_ms),
        metadata: json!({}),
    }
}

pub fn warning(code: &str, message: &str) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity: "warning".to_string(),
        scope: "document".to_string(),
        page_number: None,
        element_id: None,
        message: message.to_string(),
        recoverable: true,
        extra: HashMap::new(),
    }
}
