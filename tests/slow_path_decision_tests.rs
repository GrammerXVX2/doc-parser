use std::collections::HashMap;

use document_parser::models::config::load_model_stack_config_or_default;
use document_parser::models::domain::{DocumentDomain, DomainProfile};
use document_parser::models::router::ModelRoutingDecision;
use document_parser::models::slow_path::decide_slow_path;
use document_parser::model::{
    ContentMode, CoordinateSystem, Diagnostic, DocumentFormat, DocumentModel, DocumentProfile,
    DocumentStats, Element, ElementType, Hashes, Page, PageProfile, PageType, ProcessingStatus,
    ProcessingTrace, RuntimeInfo, SourceContainer, SourceInfo,
};
use document_parser::language::LanguageInfo;
use serde_json::json;

fn empty_model() -> DocumentModel {
    DocumentModel {
        schema_version: "1.0.0".to_string(),
        document_id: "doc_test".to_string(),
        job_id: None,
        source: SourceInfo {
            uri: "file:///tmp/test.pdf".to_string(),
            filename: "test.pdf".to_string(),
            extension: "pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            size_bytes: Some(1),
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
            format: DocumentFormat::Pdf,
            content_mode: ContentMode::Scanned,
            languages: vec!["ru".to_string()],
            language_info: LanguageInfo::default(),
            has_native_text: false,
            has_images: true,
            has_tables: true,
            has_formulas: true,
            has_ocr_required_regions: true,
            has_handwriting: false,
            has_multicolumn_layout: false,
            document_type_guess: None,
            confidence: 0.7,
        },
        stats: DocumentStats::default(),
        coordinate_system: CoordinateSystem {
            origin: "top_left".to_string(),
            unit: "px".to_string(),
            dpi: Some(300),
            normalized_to_page: true,
        },
        assets: vec![],
        pages: vec![Page {
            page_number: 1,
            page_type: PageType::DocumentPage,
            width: None,
            height: None,
            dpi: Some(300),
            rotation_degrees: 0.0,
            page_image_asset_id: None,
            page_profile: PageProfile {
                content_mode: ContentMode::Scanned,
                has_native_text: false,
                has_ocr_required_regions: true,
                has_tables: true,
                has_images: true,
                has_formulas: true,
                has_handwriting: false,
                language: Some("ru".to_string()),
                language_info: LanguageInfo::default(),
                confidence: 0.6,
            },
            elements: vec![Element {
                element_id: "e1".to_string(),
                element_type: ElementType::TextOcr,
                tag: None,
                role: None,
                reading_order: Some(1),
                global_order: Some(1),
                bbox: None,
                polygon: None,
                content: json!({"text": "placeholder table"}),
                style: json!({}),
                provenance: json!({}),
                confidence: json!({"overall": 0.4}),
                warnings: vec![],
                extra: HashMap::new(),
            }],
            text: "Договор".to_string(),
            markdown: String::new(),
            html: String::new(),
            warnings: vec![],
            extra: HashMap::new(),
        }],
        chunks: vec![],
        errors: vec![],
        warnings: vec![Diagnostic {
            code: "TEST".to_string(),
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

fn routing(profile: &str) -> ModelRoutingDecision {
    ModelRoutingDecision {
        selected_profile: profile.to_string(),
        domain_profile: DomainProfile {
            domain: DocumentDomain::Legal,
            confidence: 0.9,
            reasons: vec!["test".to_string()],
        },
        fast_ocr_backend: Some("paddleocr_ppocrv6_medium".to_string()),
        structured_backend: None,
        vlm_backend: Some("qwen3_vl".to_string()),
        layout_backend: Some("surya_layout".to_string()),
        table_backend: Some("table_transformer".to_string()),
        formula_backend: Some("pix2tex".to_string()),
        legal_ner_backend: Some("gliner_large_v2_5".to_string()),
        embedding_backend: Some("deepvk_user_bge_m3".to_string()),
        book_backends: vec![],
        slow_path_backend: Some("paddleocr_vl_1_6".to_string()),
        reasons: vec![],
    }
}

#[test]
fn low_confidence_triggers_slow_path() {
    let model = empty_model();
    let cfg = load_model_stack_config_or_default(Some(std::path::Path::new("configs/model_stack.config.jsonc")));
    let decision = decide_slow_path(&model, &routing("legal_fast"), &cfg);
    assert!(decision.should_run);
}

#[test]
fn legal_high_accuracy_always_triggers() {
    let mut model = empty_model();
    model.pages[0].elements[0].confidence = json!({"overall": 0.99});
    let cfg = load_model_stack_config_or_default(Some(std::path::Path::new("configs/model_stack.config.jsonc")));
    let decision = decide_slow_path(&model, &routing("legal_high_accuracy"), &cfg);
    assert!(decision.should_run);
}
