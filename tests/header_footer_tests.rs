use document_parser::layout::{apply_header_footer_marks, detect_repeated_headers_footers};
use document_parser::model::{ContentMode, Element, ElementType, Page, PageProfile, PageType};

fn make_page(page_number: u32, top: &str, bottom: &str) -> Page {
    Page {
        page_number,
        page_type: PageType::DocumentPage,
        width: Some(1000.0),
        height: Some(1400.0),
        dpi: None,
        rotation_degrees: 0.0,
        page_image_asset_id: None,
        page_profile: PageProfile {
            content_mode: ContentMode::Digital,
            has_native_text: true,
            has_ocr_required_regions: false,
            has_tables: false,
            has_images: false,
            has_formulas: false,
            has_handwriting: false,
            language: Some("ru".to_string()),
            language_info: Default::default(),
            confidence: 0.9,
        },
        elements: vec![
            Element {
                element_id: format!("p{}_h", page_number),
                element_type: ElementType::Paragraph,
                tag: Some("p".to_string()),
                role: None,
                reading_order: None,
                global_order: None,
                bbox: Some([40.0, 20.0, 900.0, 60.0]),
                polygon: None,
                content: serde_json::json!({"text": top}),
                style: serde_json::json!({}),
                provenance: serde_json::json!({}),
                confidence: serde_json::json!({"overall":0.9}),
                warnings: vec![],
                extra: Default::default(),
            },
            Element {
                element_id: format!("p{}_f", page_number),
                element_type: ElementType::Paragraph,
                tag: Some("p".to_string()),
                role: None,
                reading_order: None,
                global_order: None,
                bbox: Some([40.0, 1330.0, 900.0, 1380.0]),
                polygon: None,
                content: serde_json::json!({"text": bottom}),
                style: serde_json::json!({}),
                provenance: serde_json::json!({}),
                confidence: serde_json::json!({"overall":0.9}),
                warnings: vec![],
                extra: Default::default(),
            },
        ],
        text: String::new(),
        markdown: String::new(),
        html: String::new(),
        warnings: vec![],
        extra: Default::default(),
    }
}

#[test]
fn repeated_top_and_bottom_are_detected() {
    let mut pages = vec![
        make_page(1, "Компания Ромашка", "1"),
        make_page(2, "Компания Ромашка", "2"),
        make_page(3, "Компания Ромашка", "3"),
    ];

    let result = detect_repeated_headers_footers(&pages);
    assert!(!result.header_element_ids.is_empty());
    assert!(!result.footer_element_ids.is_empty());

    apply_header_footer_marks(&mut pages, &result);
    assert!(pages.iter().any(|p| {
        p.elements
            .iter()
            .any(|e| matches!(e.element_type, ElementType::Header))
    }));
    assert!(pages.iter().any(|p| {
        p.elements
            .iter()
            .any(|e| matches!(e.element_type, ElementType::Footer))
    }));
}
