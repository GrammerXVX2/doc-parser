use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::model::Element;

use super::geometry::{BBox, bbox_iou};
use super::text_similarity::text_similarity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupOptions {
    pub bbox_iou_threshold: f32,
    pub text_similarity_threshold: f32,
    pub prefer_native_text: bool,
}

impl Default for DedupOptions {
    fn default() -> Self {
        Self {
            bbox_iou_threshold: 0.5,
            text_similarity_threshold: 0.8,
            prefer_native_text: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MergeOutcome {
    pub merged_elements: Vec<Element>,
    pub removed_ocr_ids: Vec<String>,
}

pub fn merge_native_and_ocr(
    native_elements: Vec<Element>,
    ocr_elements: Vec<Element>,
    options: DedupOptions,
) -> Vec<Element> {
    merge_native_and_ocr_with_outcome(native_elements, ocr_elements, options).merged_elements
}

pub fn merge_native_and_ocr_with_outcome(
    native_elements: Vec<Element>,
    ocr_elements: Vec<Element>,
    options: DedupOptions,
) -> MergeOutcome {
    let mut merged = native_elements.clone();
    let mut removed_ocr_ids = Vec::new();

    for mut ocr in ocr_elements {
        let mut duplicate_of_native = false;

        for native in &native_elements {
            let iou = bbox_similarity(native, &ocr);
            let sim = content_similarity(native, &ocr);
            let _combined_score = 0.5 * iou + 0.5 * sim;

            if iou >= options.bbox_iou_threshold && sim >= options.text_similarity_threshold {
                duplicate_of_native = true;
                if options.prefer_native_text {
                    removed_ocr_ids.push(ocr.element_id.clone());
                    break;
                }
            }
        }

        if duplicate_of_native && options.prefer_native_text {
            continue;
        }

        ocr.extra.insert(
            "deduplication".to_string(),
            json!({
                "status": if duplicate_of_native { "kept" } else { "not_checked_or_unique" }
            }),
        );
        merged.push(ocr);
    }

    MergeOutcome {
        merged_elements: merged,
        removed_ocr_ids,
    }
}

fn bbox_similarity(a: &Element, b: &Element) -> f32 {
    let ab = a.bbox.map(BBox::from_array);
    let bb = b.bbox.map(BBox::from_array);
    match (ab, bb) {
        (Some(a), Some(b)) => bbox_iou(&a, &b),
        _ => 0.0,
    }
}

fn content_similarity(a: &Element, b: &Element) -> f32 {
    let at = element_text(a);
    let bt = element_text(b);
    text_similarity(&at, &bt)
}

fn element_text(element: &Element) -> String {
    element
        .content
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}
