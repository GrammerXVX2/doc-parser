use serde_json::json;

use crate::layout::types::LayoutRegion;
use crate::model::Page;

pub fn regions_to_json(regions: &[LayoutRegion]) -> serde_json::Value {
    serde_json::to_value(regions).unwrap_or_else(|_| json!([]))
}

pub fn page_reading_order_snapshot(page: &Page) -> serde_json::Value {
    let rows = page
        .elements
        .iter()
        .map(|el| {
            json!({
                "element_id": el.element_id,
                "type": el.element_type,
                "role": el.role,
                "reading_order": el.reading_order,
                "bbox": el.bbox,
            })
        })
        .collect::<Vec<_>>();
    json!(rows)
}
