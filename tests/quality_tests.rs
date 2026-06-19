use std::collections::HashMap;

use document_parser::model::{
    ContentMode, CoordinateSystem, Diagnostic, DocumentFormat, DocumentModel, DocumentProfile,
    DocumentStats, Element, ElementType, Hashes, Page, PageProfile, PageType,
    ProcessingStage, ProcessingStatus, ProcessingTrace, RuntimeInfo, SourceContainer, SourceInfo,
    StageStatus,
};
use document_parser::language::LanguageInfo;
use document_parser::quality::{QualityReport, generate_quality_report_from_model_path, write_quality_report};
use serde_json::json;

fn sample_model() -> DocumentModel {
    DocumentModel {
        schema_version: "1.0.0".to_string(),
        document_id: "doc_test".to_string(),
        job_id: Some("job_test".to_string()),
        source: SourceInfo {
            uri: "file:///tmp/test.txt".to_string(),
            filename: "test.txt".to_string(),
            extension: "txt".to_string(),
            mime_type: "text/plain".to_string(),
            size_bytes: Some(10),
            hashes: Hashes { sha256: None },
            uploaded_at: None,
            processed_at: None,
            container: SourceContainer {
                container_type: None,
                parent_uri: None,
                entry_path: None,
            },
        },
        document_profile: DocumentProfile {
            format: DocumentFormat::Txt,
            content_mode: ContentMode::PlainText,
            languages: vec!["ru".to_string()],
            language_info: LanguageInfo::default(),
            has_native_text: true,
            has_images: false,
            has_tables: false,
            has_formulas: false,
            has_ocr_required_regions: false,
            has_handwriting: false,
            has_multicolumn_layout: false,
            document_type_guess: None,
            confidence: 1.0,
        },
        stats: DocumentStats::default(),
        coordinate_system: CoordinateSystem {
            origin: "top_left".to_string(),
            unit: "px".to_string(),
            dpi: None,
            normalized_to_page: false,
        },
        assets: vec![],
        pages: vec![Page {
            page_number: 1,
            page_type: PageType::SyntheticTextPage,
            width: None,
            height: None,
            dpi: None,
            rotation_degrees: 0.0,
            page_image_asset_id: None,
            page_profile: PageProfile {
                content_mode: ContentMode::PlainText,
                has_native_text: true,
                has_ocr_required_regions: false,
                has_tables: false,
                has_images: false,
                has_formulas: false,
                has_handwriting: false,
                language: Some("ru".to_string()),
                language_info: LanguageInfo::default(),
                confidence: 1.0,
            },
            elements: vec![
                Element {
                    element_id: "e1".to_string(),
                    element_type: ElementType::Text,
                    tag: None,
                    role: None,
                    reading_order: Some(1),
                    global_order: Some(1),
                    bbox: None,
                    polygon: None,
                    content: json!({"text": "Повтор текста"}),
                    style: json!({}),
                    provenance: json!({}),
                    confidence: json!({"overall": 1.0}),
                    warnings: vec![],
                    extra: HashMap::new(),
                },
                Element {
                    element_id: "e2".to_string(),
                    element_type: ElementType::Text,
                    tag: None,
                    role: None,
                    reading_order: Some(2),
                    global_order: Some(2),
                    bbox: None,
                    polygon: None,
                    content: json!({"text": "Повтор текста"}),
                    style: json!({}),
                    provenance: json!({}),
                    confidence: json!({"overall": 1.0}),
                    warnings: vec![],
                    extra: HashMap::new(),
                },
                Element {
                    element_id: "e3".to_string(),
                    element_type: ElementType::TextOcr,
                    tag: None,
                    role: None,
                    reading_order: Some(3),
                    global_order: Some(3),
                    bbox: None,
                    polygon: None,
                    content: json!({"text": "OCR"}),
                    style: json!({}),
                    provenance: json!({}),
                    confidence: json!({"overall": 0.3}),
                    warnings: vec![],
                    extra: HashMap::new(),
                },
            ],
            text: "Повтор текста\nПовтор текста\nOCR".to_string(),
            markdown: String::new(),
            html: String::new(),
            warnings: vec![],
            extra: HashMap::new(),
        }],
        chunks: vec![],
        errors: vec![],
        warnings: vec![Diagnostic {
            code: "WARN".to_string(),
            severity: "warning".to_string(),
            scope: "document".to_string(),
            page_number: None,
            element_id: None,
            message: "test".to_string(),
            recoverable: true,
            extra: HashMap::new(),
        }],
        processing: ProcessingTrace {
            pipeline_version: "1.0.0".to_string(),
            status: ProcessingStatus::Ok,
            stages: vec![ProcessingStage {
                name: "extract".to_string(),
                status: StageStatus::Ok,
                tool: "txt_extractor".to_string(),
                duration_ms: Some(1),
                metadata: json!({}),
            }],
            total_duration_ms: Some(1),
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

#[test]
fn quality_report_generated_and_written() {
    let model = sample_model();
    let report = QualityReport::from_model(&model);

    assert_eq!(report.document_id, "doc_test");
    assert_eq!(report.pages, 1);
    assert!(report.duplicate_text_ratio > 0.0);
    assert!(report.low_confidence_ocr_ratio > 0.0);

    let dir = std::env::temp_dir().join(format!("quality_test_{}", uuid::Uuid::new_v4()));
    let (json_path, md_path) = write_quality_report(&report, &dir).unwrap();
    assert!(json_path.exists());
    assert!(md_path.exists());
}

#[test]
fn quality_report_from_model_path_works() {
    let model = sample_model();
    let dir = std::env::temp_dir().join(format!("quality_model_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("model.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&model).unwrap()).unwrap();

    let report = generate_quality_report_from_model_path(&path).unwrap();
    assert_eq!(report.document_id, "doc_test");
}
