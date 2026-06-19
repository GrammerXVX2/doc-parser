use crate::config::PipelineConfig;
use crate::model::ContentMode;

use super::types::{PdfClassification, PdfPageClassification, PdfPageContentMode};

pub fn classify_pdf_by_text(
    text: &str,
    encrypted: bool,
    page_count_hint: Option<usize>,
    pipeline_config: Option<&PipelineConfig>,
) -> PdfClassification {
    let native_text_min_chars = pipeline_config
        .and_then(|c| c.pipeline.pdf.get("native_text_min_chars_per_page"))
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;

    let mut pages_raw = split_pdf_text_to_pages(text);
    if let Some(hint) = page_count_hint {
        while pages_raw.len() < hint {
            pages_raw.push(String::new());
        }
    }

    let mut pages = Vec::new();
    for (index, page_text) in pages_raw.iter().enumerate() {
        let chars = page_text.chars().filter(|c| !c.is_whitespace()).count();
        let has_native_text = chars >= native_text_min_chars;

        let content_mode = if has_native_text {
            PdfPageContentMode::Digital
        } else if chars == 0 {
            PdfPageContentMode::Scanned
        } else {
            PdfPageContentMode::Unknown
        };

        let requires_ocr = !has_native_text;

        pages.push(PdfPageClassification {
            page_number: index + 1,
            content_mode,
            has_native_text,
            native_text_chars: chars,
            has_images: false,
            image_area_ratio: None,
            requires_ocr,
            confidence: if has_native_text { 0.9 } else { 0.7 },
        });
    }

    let document_content_mode = detect_document_mode(&pages);

    PdfClassification {
        page_count: pages.len().max(page_count_hint.unwrap_or(1)).max(1),
        encrypted,
        pages,
        document_content_mode,
    }
}

fn detect_document_mode(pages: &[PdfPageClassification]) -> ContentMode {
    if pages.is_empty() {
        return ContentMode::Unknown;
    }

    let digital = pages
        .iter()
        .filter(|p| matches!(p.content_mode, PdfPageContentMode::Digital))
        .count();
    let scanned = pages
        .iter()
        .filter(|p| matches!(p.content_mode, PdfPageContentMode::Scanned))
        .count();

    if digital > 0 && scanned > 0 {
        ContentMode::Hybrid
    } else if digital > 0 {
        ContentMode::Digital
    } else if scanned > 0 {
        ContentMode::Scanned
    } else {
        ContentMode::Unknown
    }
}

pub fn split_pdf_text_to_pages(text: &str) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return vec![String::new()];
    }

    if trimmed.contains('\u{000c}') {
        return trimmed
            .split('\u{000c}')
            .map(|s| s.trim().to_string())
            .collect();
    }

    vec![trimmed.to_string()]
}
