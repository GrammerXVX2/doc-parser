use std::collections::HashMap;

use serde_json::json;

use document_parser::merge::{DedupOptions, merge_native_and_ocr_with_outcome};
use document_parser::model::{Element, ElementType};

fn make_element(id: &str, element_type: ElementType, text: &str, bbox: [f32; 4]) -> Element {
    Element {
        element_id: id.to_string(),
        element_type,
        tag: None,
        role: Some("paragraph".to_string()),
        reading_order: None,
        global_order: None,
        bbox: Some(bbox),
        polygon: None,
        content: json!({
            "text": text,
            "html": null,
            "markdown": text,
            "normalized_text": text.to_lowercase(),
            "raw": null,
        }),
        style: json!({}),
        provenance: json!({"method": "test"}),
        confidence: json!({"overall": 1.0}),
        warnings: vec![],
        extra: HashMap::new(),
    }
}

#[test]
fn same_bbox_and_same_text_removes_ocr_duplicate() {
    let native = vec![make_element(
        "p1_native_1",
        ElementType::Text,
        "Invoice number: 12345",
        [100.0, 100.0, 400.0, 140.0],
    )];
    let ocr = vec![make_element(
        "p1_ocr_1",
        ElementType::TextOcr,
        "Invoice number: 12345",
        [100.0, 100.0, 400.0, 140.0],
    )];

    let outcome = merge_native_and_ocr_with_outcome(native, ocr, DedupOptions::default());
    assert_eq!(outcome.merged_elements.len(), 1);
    assert_eq!(outcome.removed_ocr_ids, vec!["p1_ocr_1".to_string()]);
}

#[test]
fn same_bbox_but_different_text_keeps_ocr() {
    let native = vec![make_element(
        "p1_native_1",
        ElementType::Text,
        "Invoice number: 12345",
        [100.0, 100.0, 400.0, 140.0],
    )];
    let ocr = vec![make_element(
        "p1_ocr_1",
        ElementType::TextOcr,
        "Total: $100.00",
        [100.0, 100.0, 400.0, 140.0],
    )];

    let outcome = merge_native_and_ocr_with_outcome(native, ocr, DedupOptions::default());
    assert_eq!(outcome.merged_elements.len(), 2);
    assert!(outcome.removed_ocr_ids.is_empty());
}

#[test]
fn different_bbox_same_text_keeps_ocr_by_default() {
    let native = vec![make_element(
        "p1_native_1",
        ElementType::Text,
        "Invoice number: 12345",
        [100.0, 100.0, 400.0, 140.0],
    )];
    let ocr = vec![make_element(
        "p1_ocr_1",
        ElementType::TextOcr,
        "Invoice number: 12345",
        [600.0, 600.0, 900.0, 640.0],
    )];

    let outcome = merge_native_and_ocr_with_outcome(native, ocr, DedupOptions::default());
    assert_eq!(outcome.merged_elements.len(), 2);
    assert!(outcome.removed_ocr_ids.is_empty());
}

#[test]
fn punctuation_and_case_variants_still_dedup() {
    let native = vec![make_element(
        "p1_native_1",
        ElementType::Text,
        "Invoice number: 12345",
        [100.0, 100.0, 400.0, 140.0],
    )];
    let ocr = vec![make_element(
        "p1_ocr_1",
        ElementType::TextOcr,
        "invoice number 12345!!!",
        [100.0, 100.0, 400.0, 140.0],
    )];

    let outcome = merge_native_and_ocr_with_outcome(native, ocr, DedupOptions::default());
    assert_eq!(outcome.merged_elements.len(), 1);
    assert_eq!(outcome.removed_ocr_ids.len(), 1);
}
