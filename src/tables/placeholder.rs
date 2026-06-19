use std::collections::HashMap;

use serde_json::json;

use crate::extractors::{default_confidence, empty_style};
use crate::model::{Diagnostic, Element, ElementType};
use crate::utils::geometry::BBox;

pub fn create_scanned_table_placeholder(
    page_number: usize,
    region_id: &str,
    bbox: BBox,
    confidence: f32,
    source: &str,
) -> Element {
    let text = "[Таблица обнаружена на скане, структура еще не распознана]";

    let mut extra = HashMap::new();
    extra.insert("rows".to_string(), json!(0));
    extra.insert("columns".to_string(), json!(0));
    extra.insert(
        "table_structure".to_string(),
        json!({"extraction_method": "detected_placeholder"}),
    );
    extra.insert("detected_region_id".to_string(), json!(region_id));
    extra.insert("detector_source".to_string(), json!(source));

    Element {
        element_id: format!("p{}_table_placeholder_{}", page_number, region_id),
        element_type: ElementType::Table,
        tag: Some("table_placeholder".to_string()),
        role: Some("scanned_table_placeholder".to_string()),
        reading_order: None,
        global_order: None,
        bbox: Some(bbox.to_array()),
        polygon: None,
        content: json!({
            "text": text,
            "markdown": "",
            "html": "",
            "csv": "",
            "normalized_text": text,
            "raw": null,
        }),
        style: empty_style(),
        provenance: json!({
            "method": "inferred",
            "tool": source,
            "stage": "scanned_table_detection",
            "source_ref": {
                "kind": "region",
                "value": region_id,
            }
        }),
        confidence: {
            let mut conf = default_confidence();
            conf["overall"] = json!(confidence);
            conf["structure"] = json!(0.0);
            conf
        },
        warnings: vec![Diagnostic {
            code: "TABLE_PLACEHOLDER_CREATED".to_string(),
            severity: "warning".to_string(),
            scope: "element".to_string(),
            page_number: Some(page_number as u32),
            element_id: None,
            message:
                "На странице обнаружена таблица, но структура таблицы пока не распознана."
                    .to_string(),
            recoverable: true,
            extra: HashMap::new(),
        }],
        extra,
    }
}
