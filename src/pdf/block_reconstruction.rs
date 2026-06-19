use std::collections::HashMap;

use serde_json::json;

use crate::extractors::{default_confidence, empty_style};
use crate::model::{Element, ElementType};
use crate::pdf::layout_hints::infer_block_role_hint;
use crate::pdf::spans::{PdfTextBlock, PdfTextLine, PdfTextReconstructionOptions, PdfTextSpan};
use crate::utils::geometry::BBox;
use crate::utils::russian_text::normalize_russian_text;

pub fn merge_spans_into_lines(
    mut spans: Vec<PdfTextSpan>,
    options: PdfTextReconstructionOptions,
) -> Vec<PdfTextLine> {
    spans.sort_by(|a, b| {
        a.bbox
            .y0
            .partial_cmp(&b.bbox.y0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.bbox.x0.partial_cmp(&b.bbox.x0).unwrap_or(std::cmp::Ordering::Equal))
    });

    let mut lines: Vec<PdfTextLine> = Vec::new();
    for span in spans {
        let mut placed = false;
        for line in &mut lines {
            let dy = (line.baseline_y - span.bbox.y0).abs();
            let line_height = line.bbox.height().max(10.0);
            if dy <= line_height * options.line_y_tolerance.max(0.005) * 20.0 {
                let last_x = line
                    .spans
                    .last()
                    .map(|s| s.bbox.x1)
                    .unwrap_or(line.bbox.x0);
                let gap = (span.bbox.x0 - last_x).max(0.0);
                let avg_height = (line.bbox.height() + span.bbox.height()) / 2.0;
                if gap <= avg_height * (options.word_gap_ratio * 4.0 + 1.0) {
                    line.spans.push(span.clone());
                    line.spans
                        .sort_by(|a, b| a.bbox.x0.partial_cmp(&b.bbox.x0).unwrap_or(std::cmp::Ordering::Equal));
                    line.text = build_line_text(&line.spans, options.word_gap_ratio);
                    line.bbox = merge_bbox(&line.bbox, &span.bbox);
                    line.baseline_y = line.bbox.y0;
                    placed = true;
                    break;
                }
            }
        }

        if !placed {
            lines.push(PdfTextLine {
                spans: vec![span.clone()],
                text: span.text.clone(),
                bbox: span.bbox,
                baseline_y: span.bbox.y0,
            });
        }
    }

    lines.sort_by(|a, b| a.bbox.y0.partial_cmp(&b.bbox.y0).unwrap_or(std::cmp::Ordering::Equal));
    lines
}

pub fn merge_lines_into_blocks(
    mut lines: Vec<PdfTextLine>,
    options: PdfTextReconstructionOptions,
) -> Vec<PdfTextBlock> {
    lines.sort_by(|a, b| a.bbox.y0.partial_cmp(&b.bbox.y0).unwrap_or(std::cmp::Ordering::Equal));

    let mut blocks: Vec<PdfTextBlock> = Vec::new();
    for line in lines {
        let mut placed = false;
        for block in &mut blocks {
            let last_line = match block.lines.last() {
                Some(v) => v,
                None => continue,
            };
            let line_height = last_line.bbox.height().max(10.0);
            let vertical_gap = (line.bbox.y0 - last_line.bbox.y1).max(0.0);
            let x_overlap = overlap_1d(last_line.bbox.x0, last_line.bbox.x1, line.bbox.x0, line.bbox.x1);
            if vertical_gap <= line_height * options.paragraph_gap_ratio && x_overlap > 0.0 {
                block.lines.push(line.clone());
                block.bbox = merge_bbox(&block.bbox, &line.bbox);
                block.text = block
                    .lines
                    .iter()
                    .map(|ln| ln.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                placed = true;
                break;
            }
        }

        if !placed {
            blocks.push(PdfTextBlock {
                lines: vec![line.clone()],
                text: line.text.clone(),
                bbox: line.bbox,
                role_hint: None,
            });
        }
    }

    let median_font = median_font_size(&blocks);
    for block in &mut blocks {
        block.role_hint = infer_block_role_hint(block, median_font);
    }

    blocks
}

pub fn pdf_blocks_to_elements(blocks: Vec<PdfTextBlock>) -> Vec<Element> {
    let mut elements = Vec::new();

    for (idx, block) in blocks.into_iter().enumerate() {
        let raw_text = block.text.trim().to_string();
        if raw_text.is_empty() {
            continue;
        }

        let element_type = if block.role_hint.as_deref() == Some("section_title") {
            ElementType::Heading
        } else {
            ElementType::Paragraph
        };

        let normalized = if matches!(element_type, ElementType::Code) {
            raw_text.clone()
        } else {
            normalize_russian_text(&raw_text)
        };

        let mut extra = HashMap::new();
        if let Some(role) = &block.role_hint {
            extra.insert("role_hint".to_string(), json!(role));
        }

        elements.push(Element {
            element_id: format!("pdf_block_{}", idx + 1),
            element_type: element_type.clone(),
            tag: Some(if matches!(element_type, ElementType::Heading) {
                "h1"
            } else {
                "p"
            }
            .to_string()),
            role: block.role_hint.clone(),
            reading_order: Some((idx + 1) as u32),
            global_order: None,
            bbox: Some(block.bbox.to_array()),
            polygon: None,
            content: json!({
                "text": normalized,
                "markdown": if matches!(element_type, ElementType::Heading) {
                    format!("# {}", normalized)
                } else {
                    normalized
                },
                "html": null,
                "normalized_text": normalize_russian_text(&raw_text),
                "raw": raw_text,
            }),
            style: empty_style(),
            provenance: json!({
                "method": "native",
                "tool": "pdf_block_reconstructor",
                "stage": "pdf_text_reconstruction"
            }),
            confidence: {
                let mut conf = default_confidence();
                conf["layout"] = json!(0.85);
                conf
            },
            warnings: vec![],
            extra,
        });
    }

    elements
}

fn build_line_text(spans: &[PdfTextSpan], gap_ratio: f32) -> String {
    if spans.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    let mut prev_x1 = spans[0].bbox.x0;
    let avg_height = spans.iter().map(|s| s.bbox.height()).sum::<f32>() / spans.len() as f32;
    let threshold = avg_height * (gap_ratio * 2.0 + 0.5);

    for span in spans {
        let gap = span.bbox.x0 - prev_x1;
        if !out.is_empty() && gap > threshold {
            out.push(' ');
        }
        out.push_str(span.text.trim());
        prev_x1 = span.bbox.x1;
    }

    out
}

fn merge_bbox(a: &BBox, b: &BBox) -> BBox {
    BBox {
        x0: a.x0.min(b.x0),
        y0: a.y0.min(b.y0),
        x1: a.x1.max(b.x1),
        y1: a.y1.max(b.y1),
    }
}

fn overlap_1d(a0: f32, a1: f32, b0: f32, b1: f32) -> f32 {
    (a1.min(b1) - a0.max(b0)).max(0.0)
}

fn median_font_size(blocks: &[PdfTextBlock]) -> Option<f32> {
    let mut values = blocks
        .iter()
        .flat_map(|b| b.lines.iter())
        .flat_map(|l| l.spans.iter())
        .filter_map(|s| s.font_size)
        .collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    Some(if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    })
}
