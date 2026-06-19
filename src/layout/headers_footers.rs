use std::collections::{HashMap, HashSet};

use crate::model::{ElementType, Page};

#[derive(Debug, Clone, Default)]
pub struct HeaderFooterDetectionResult {
    pub header_element_ids: Vec<String>,
    pub footer_element_ids: Vec<String>,
}

pub fn detect_repeated_headers_footers(pages: &[Page]) -> HeaderFooterDetectionResult {
    if pages.len() < 2 {
        return HeaderFooterDetectionResult::default();
    }

    let mut top_map: HashMap<String, HashSet<u32>> = HashMap::new();
    let mut bottom_map: HashMap<String, HashSet<u32>> = HashMap::new();
    let mut element_by_text_top: HashMap<String, Vec<String>> = HashMap::new();
    let mut element_by_text_bottom: HashMap<String, Vec<String>> = HashMap::new();

    for page in pages {
        let page_height = page
            .height
            .or_else(|| {
                page
                    .elements
                    .iter()
                    .filter_map(|e| e.bbox.map(|b| b[3]))
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            })
            .unwrap_or(1400.0);

        for element in &page.elements {
            let text = element
                .content
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .trim()
                .to_string();
            let Some(bbox) = element.bbox else {
                continue;
            };
            if text.is_empty() {
                continue;
            }

            let key = normalize_text_for_repeat(&text);
            if key.is_empty() {
                continue;
            }

            if bbox[1] <= page_height * 0.12 {
                top_map
                    .entry(key.clone())
                    .or_default()
                    .insert(page.page_number);
                element_by_text_top
                    .entry(key.clone())
                    .or_default()
                    .push(element.element_id.clone());
            }

            if bbox[3] >= page_height * 0.88 {
                bottom_map
                    .entry(key.clone())
                    .or_default()
                    .insert(page.page_number);
                element_by_text_bottom
                    .entry(key)
                    .or_default()
                    .push(element.element_id.clone());
            }
        }
    }

    let min_pages = (pages.len() as f32 * 0.5).ceil() as usize;

    let mut headers = Vec::new();
    for (text, page_set) in top_map {
        if page_set.len() >= min_pages.max(2) {
            if let Some(ids) = element_by_text_top.get(&text) {
                headers.extend(ids.iter().cloned());
            }
        }
    }

    let mut footers = Vec::new();
    for (text, page_set) in bottom_map {
        let is_page_number = is_page_number_like(&text);
        if page_set.len() >= min_pages.max(2) || is_page_number {
            if let Some(ids) = element_by_text_bottom.get(&text) {
                footers.extend(ids.iter().cloned());
            }
        }
    }

    HeaderFooterDetectionResult {
        header_element_ids: headers,
        footer_element_ids: footers,
    }
}

pub fn apply_header_footer_marks(pages: &mut [Page], result: &HeaderFooterDetectionResult) {
    let headers = result.header_element_ids.iter().cloned().collect::<HashSet<_>>();
    let footers = result.footer_element_ids.iter().cloned().collect::<HashSet<_>>();

    for page in pages {
        for element in &mut page.elements {
            if headers.contains(&element.element_id) {
                element.element_type = ElementType::Header;
                element.role = Some("repeated_header".to_string());
            }
            if footers.contains(&element.element_id) {
                let text = element
                    .content
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                element.element_type = ElementType::Footer;
                element.role = Some(if is_page_number_like(text) {
                    "page_number"
                } else {
                    "repeated_footer"
                }
                .to_string());
            }
        }
    }
}

fn normalize_text_for_repeat(text: &str) -> String {
    text.to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_whitespace() { ' ' } else { ch })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_page_number_like(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed.chars().all(|c| c.is_ascii_digit())
        || trimmed.starts_with("стр") && trimmed.chars().any(|c| c.is_ascii_digit())
}
