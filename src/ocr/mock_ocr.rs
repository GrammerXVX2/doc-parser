use std::collections::HashMap;
use std::fs;

use anyhow::Context;
use serde_json::json;

use crate::extractors::empty_style;
use crate::ocr::postprocessing::{
    ParagraphGroupingOptions, filter_low_confidence, group_ocr_lines_into_paragraphs,
    sort_text_regions_reading_order,
};
use crate::model::{Element, ElementType};
use crate::utils::russian_text::normalize_russian_text;
use crate::utils::geometry::BBox;

use super::traits::OcrPipeline;
use super::types::{OcrFixtureLine, OcrPageInput, RecognizedText, TextRegion};

#[derive(Debug, Clone)]
pub struct MockOcrConfig {
    pub fixture_based: bool,
    pub deterministic_fallback: bool,
}

impl Default for MockOcrConfig {
    fn default() -> Self {
        Self {
            fixture_based: true,
            deterministic_fallback: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MockOcrPipeline {
    pub config: MockOcrConfig,
}

impl OcrPipeline for MockOcrPipeline {
    fn run_page_ocr(&self, input: OcrPageInput) -> anyhow::Result<Vec<Element>> {
        let mut recognized = Vec::new();
        if self.config.fixture_based {
            if let Some(lines) = load_fixture_lines(&input)? {
                recognized = lines;
            }
        }

        if recognized.is_empty() && self.config.deterministic_fallback {
            recognized = deterministic_lines(&input);
        }

        if recognized.is_empty() {
            return Ok(vec![]);
        }

        let sorted = sort_text_regions_reading_order(recognized, input.width as f32, input.height as f32);
        let (kept, low_conf) = filter_low_confidence(sorted, 0.5, false);
        let paragraphs = group_ocr_lines_into_paragraphs(kept, ParagraphGroupingOptions::default());

        let mut elements = Vec::new();
        for (idx, paragraph) in paragraphs.into_iter().enumerate() {
            let mut element = ocr_element(
                input.page_number,
                idx + 1,
                &normalize_russian_text(&paragraph.text),
                paragraph.bbox,
                paragraph.confidence,
            );

            let lines_json = paragraph
                .lines
                .iter()
                .map(|line| {
                    json!({
                        "text": normalize_russian_text(&line.text),
                        "bbox": line.region.bbox.to_array(),
                        "confidence": line.confidence,
                        "language": line.language.clone().unwrap_or_else(|| "ru".to_string()),
                    })
                })
                .collect::<Vec<_>>();

            if let Some(ocr) = element.extra.get_mut("ocr") {
                ocr["lines"] = json!(lines_json);
            }

            if paragraph.confidence < 0.5 {
                element.warnings.push(crate::model::Diagnostic {
                    code: "LOW_OCR_CONFIDENCE".to_string(),
                    severity: "warning".to_string(),
                    scope: "element".to_string(),
                    page_number: Some(input.page_number as u32),
                    element_id: Some(element.element_id.clone()),
                    message: "Низкая уверенность OCR-распознавания.".to_string(),
                    recoverable: true,
                    extra: HashMap::new(),
                });
            }

            element.extra.insert("language".to_string(), json!("ru"));
            elements.push(element);
        }

        for low in low_conf {
            if let Some(last) = elements.last_mut() {
                last.warnings.push(crate::model::Diagnostic {
                    code: "LOW_OCR_CONFIDENCE".to_string(),
                    severity: "warning".to_string(),
                    scope: "element".to_string(),
                    page_number: Some(input.page_number as u32),
                    element_id: Some(last.element_id.clone()),
                    message: format!(
                        "Низкая уверенность OCR-распознавания: {}",
                        normalize_russian_text(&low.text)
                    ),
                    recoverable: true,
                    extra: HashMap::new(),
                });
            }
        }

        return Ok(elements);
    }
}

fn load_fixture_lines(input: &OcrPageInput) -> anyhow::Result<Option<Vec<RecognizedText>>> {
    let fixture_path = format!("{}.ocr.json", input.image_path.to_string_lossy());
    let fixture_path = std::path::PathBuf::from(fixture_path);

    if !fixture_path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&fixture_path)
        .with_context(|| format!("failed to read OCR fixture: {}", fixture_path.display()))?;
    let lines: Vec<OcrFixtureLine> = serde_json::from_str(&raw)
        .with_context(|| format!("invalid OCR fixture JSON: {}", fixture_path.display()))?;

    let mut recognized = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        let _ = idx;
        recognized.push(RecognizedText {
            text: line.text.clone(),
            region: TextRegion {
                bbox: BBox::from_array(line.bbox),
                polygon: None,
                confidence: line.confidence,
                orientation_degrees: 0.0,
            },
            confidence: line.confidence,
            language: Some("ru".to_string()),
            det_confidence: Some(line.confidence),
            crop_asset_id: None,
        });
    }

    Ok(Some(recognized))
}

fn deterministic_lines(input: &OcrPageInput) -> Vec<RecognizedText> {
    let line1 = format!("Mock OCR text for page {}", input.page_number);
    let line2 = format!("Mock OCR secondary line for page {}", input.page_number);

    vec![
        RecognizedText {
            text: line1,
            region: TextRegion {
                bbox: BBox {
                    x0: 100.0,
                    y0: 120.0,
                    x1: 800.0,
                    y1: 160.0,
                },
                polygon: None,
                confidence: 0.95,
                orientation_degrees: 0.0,
            },
            confidence: 0.95,
            language: Some("ru".to_string()),
            det_confidence: Some(0.95),
            crop_asset_id: None,
        },
        RecognizedText {
            text: line2,
            region: TextRegion {
                bbox: BBox {
                    x0: 100.0,
                    y0: 200.0,
                    x1: 900.0,
                    y1: 240.0,
                },
                polygon: None,
                confidence: 0.45,
                orientation_degrees: 0.0,
            },
            confidence: 0.45,
            language: Some("ru".to_string()),
            det_confidence: Some(0.45),
            crop_asset_id: None,
        },
    ]
}

fn ocr_element(page_number: usize, idx: usize, text: &str, bbox: BBox, confidence: f32) -> Element {
    let mut element = Element {
        element_id: format!("p{}_ocr_{}", page_number, idx),
        element_type: ElementType::TextOcr,
        tag: Some("ocr".to_string()),
        role: Some("paragraph".to_string()),
        reading_order: None,
        global_order: None,
        bbox: Some(bbox.to_array()),
        polygon: None,
        content: json!({
            "text": text,
            "html": null,
            "markdown": text,
            "normalized_text": normalize_russian_text(text),
            "raw": null,
        }),
        style: empty_style(),
        provenance: json!({
            "method": "ocr",
            "tool": "mock_ocr",
            "stage": "ocr_recognition"
        }),
        confidence: json!({
            "overall": confidence,
            "text": confidence,
            "layout": confidence,
            "language": 0.9
        }),
        warnings: vec![],
        extra: HashMap::new(),
    };

    element.extra.insert(
        "ocr".to_string(),
        json!({
            "engine": "mock_ocr",
            "det_confidence": confidence,
            "rec_confidence": confidence,
            "language_hint": "ru"
        }),
    );

    element
}
