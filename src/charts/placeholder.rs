use std::collections::HashMap;

use serde_json::json;

use crate::extractors::{default_confidence, empty_style};
use crate::model::{Element, ElementType};
use crate::utils::geometry::BBox;

pub fn create_chart_placeholder(page_number: usize, bbox: BBox) -> Element {
    Element {
        element_id: format!("p{}_chart_placeholder", page_number),
        element_type: ElementType::Chart,
        tag: Some("chart_placeholder".to_string()),
        role: Some("chart_placeholder".to_string()),
        reading_order: None,
        global_order: None,
        bbox: Some(bbox.to_array()),
        polygon: None,
        content: json!({
            "text": "[Обнаружена диаграмма, извлечение метаданных пока ограничено]",
            "markdown": "",
            "html": null,
            "normalized_text": "",
            "raw": null,
        }),
        style: empty_style(),
        provenance: json!({
            "method": "inferred",
            "tool": "chart_placeholder",
            "stage": "layout_detection"
        }),
        confidence: default_confidence(),
        warnings: vec![],
        extra: HashMap::new(),
    }
}
