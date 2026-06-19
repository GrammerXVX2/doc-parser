use std::collections::HashMap;

use serde_json::json;

use crate::extractors::{default_confidence, empty_style};
use crate::model::{Diagnostic, Element, ElementType};
use crate::utils::geometry::BBox;

pub fn create_formula_placeholder(
    page_number: usize,
    region_id: &str,
    bbox: BBox,
    confidence: f32,
    source: &str,
) -> Element {
    let text = "[Формула обнаружена, распознавание еще не выполнено]";

    Element {
        element_id: format!("p{}_formula_placeholder_{}", page_number, region_id),
        element_type: ElementType::Formula,
        tag: Some("formula_placeholder".to_string()),
        role: Some("formula_placeholder".to_string()),
        reading_order: None,
        global_order: None,
        bbox: Some(bbox.to_array()),
        polygon: None,
        content: json!({
            "text": text,
            "latex": null,
            "mathml": null,
            "markdown": text,
            "normalized_text": text,
            "raw": null,
        }),
        style: empty_style(),
        provenance: json!({
            "method": "inferred",
            "tool": source,
            "stage": "formula_detection",
            "source_ref": {
                "kind": "region",
                "value": region_id,
            }
        }),
        confidence: {
            let mut conf = default_confidence();
            conf["overall"] = json!(confidence);
            conf
        },
        warnings: vec![Diagnostic {
            code: "FORMULA_PLACEHOLDER_CREATED".to_string(),
            severity: "warning".to_string(),
            scope: "element".to_string(),
            page_number: Some(page_number as u32),
            element_id: None,
            message: "Формула обнаружена, но распознавание пока не выполнено.".to_string(),
            recoverable: true,
            extra: HashMap::new(),
        }],
        extra: {
            let mut extra = HashMap::new();
            extra.insert("latex_source".to_string(), json!(null));
            extra.insert("format".to_string(), json!("unknown"));
            extra.insert("detected_region_id".to_string(), json!(region_id));
            extra
        },
    }
}
