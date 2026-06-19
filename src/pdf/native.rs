use std::collections::HashMap;

use serde_json::json;

use crate::extractors::{empty_style, provenance};
use crate::model::{Element, ElementType};
use crate::pdf::spans::PdfTextSpan;
use crate::utils::geometry::BBox;
use crate::utils::russian_text::normalize_russian_text;

use super::types::{PdfClassification, PdfPageNativeText};

pub fn extract_native_pages(text: &str, classification: &PdfClassification) -> Vec<PdfPageNativeText> {
    let pages = super::classifier::split_pdf_text_to_pages(text);

    if pages.is_empty() {
        return vec![PdfPageNativeText {
            page_number: 1,
            text: String::new(),
            synthetic_bbox: Some([0.0, 0.0, 1000.0, 1400.0]),
        }];
    }

    pages
        .iter()
        .enumerate()
        .map(|(i, p)| PdfPageNativeText {
            page_number: i + 1,
            text: p.clone(),
            synthetic_bbox: Some([0.0, 0.0, 1000.0, 1400.0]),
        })
        .take(classification.page_count.max(1))
        .collect()
}

pub fn native_text_to_elements(page_number: usize, text: &str) -> Vec<Element> {
    let mut elements = Vec::new();
    let mut y = 20.0_f32;
    let mut order = 1_u32;

    for block in text.split("\n\n") {
        let paragraph = block
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        if paragraph.is_empty() {
            continue;
        }

        let element_type = if paragraph.len() < 80
            && paragraph
                .chars()
                .all(|c| !c.is_lowercase() || c.is_ascii_digit())
        {
            ElementType::Heading
        } else {
            ElementType::Text
        };

        elements.push(Element {
            element_id: format!("p{}_native_{}", page_number, order),
            element_type,
            tag: Some("pdf_text".to_string()),
            role: Some("paragraph".to_string()),
            reading_order: Some(order),
            global_order: None,
            bbox: Some([0.0, y, 1000.0, y + 22.0]),
            polygon: None,
            content: json!({
                "text": paragraph,
                "html": null,
                "markdown": paragraph,
                "normalized_text": normalize_russian_text(&paragraph),
                "raw": null,
            }),
            style: empty_style(),
            provenance: provenance("pdf_native_extractor", "pdf_native_extraction", "page", &page_number.to_string()),
            confidence: json!({
                "overall": 0.95,
                "text": 0.98,
                "layout": 0.7,
                "language": 0.9
            }),
            warnings: vec![],
            extra: HashMap::new(),
        });

        y += 24.0;
        order += 1;
    }

    if elements.is_empty() {
        return vec![];
    }

    elements
}

pub fn text_to_synthetic_spans(page_number: usize, text: &str) -> Vec<PdfTextSpan> {
    let mut spans = Vec::new();
    let mut y = 40.0_f32;

    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            y += 20.0;
            continue;
        }
        let width = (trimmed.chars().count() as f32 * 7.0).max(40.0);
        let x0 = 40.0 + ((idx % 2) as f32 * 4.0);
        let font_size = if idx == 0 && trimmed.chars().count() < 120 {
            Some(18.0)
        } else {
            Some(12.0)
        };

        spans.push(PdfTextSpan {
            text: trimmed.to_string(),
            bbox: BBox {
                x0,
                y0: y,
                x1: (x0 + width).min(980.0),
                y1: y + 14.0,
            },
            font_size,
            font_name: Some("Times".to_string()),
            bold: Some(idx == 0),
            italic: Some(false),
        });
        y += 18.0;
    }

    if spans.is_empty() {
        spans.push(PdfTextSpan {
            text: String::new(),
            bbox: BBox {
                x0: 0.0,
                y0: 0.0,
                x1: 1000.0,
                y1: 20.0,
            },
            font_size: Some(12.0),
            font_name: Some("Times".to_string()),
            bold: Some(false),
            italic: Some(false),
        });
    }

    let _ = page_number;
    spans
}
