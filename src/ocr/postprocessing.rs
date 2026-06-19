use crate::ml::OnnxOutputs;
use crate::ocr::preprocessing::PreprocessedImage;
use crate::ocr::types::{RecognizedText, TextRegion};
use crate::utils::geometry::BBox;

#[derive(Debug, Clone, Copy)]
pub enum DetectionModelKind {
    PaddleDb,
    GenericBoxes,
}

pub trait DetectionPostProcessor {
    fn postprocess(
        &self,
        outputs: OnnxOutputs,
        preprocess_info: &PreprocessedImage,
    ) -> anyhow::Result<Vec<TextRegion>>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct GenericBoxesPostProcessor;

impl DetectionPostProcessor for GenericBoxesPostProcessor {
    fn postprocess(
        &self,
        outputs: OnnxOutputs,
        preprocess_info: &PreprocessedImage,
    ) -> anyhow::Result<Vec<TextRegion>> {
        if let Some(tensor) = outputs.first() {
            let mut out = Vec::new();
            for row in tensor.data.chunks(5) {
                if row.len() < 5 {
                    continue;
                }
                let x0 = row[0];
                let y0 = row[1];
                let x1 = row[2];
                let y1 = row[3];
                let confidence = row[4].clamp(0.0, 1.0);
                if x1 <= x0 || y1 <= y0 {
                    continue;
                }
                out.push(TextRegion {
                    bbox: BBox { x0, y0, x1, y1 },
                    polygon: None,
                    confidence,
                    orientation_degrees: 0.0,
                });
            }
            if !out.is_empty() {
                return Ok(out);
            }
        }

        Ok(vec![TextRegion {
            bbox: BBox {
                x0: 0.0,
                y0: 0.0,
                x1: preprocess_info.original_width as f32,
                y1: preprocess_info.original_height as f32,
            },
            polygon: None,
            confidence: 0.8,
            orientation_degrees: 0.0,
        }])
    }
}

#[derive(Debug, Clone)]
pub struct ParagraphGroupingOptions {
    pub y_tolerance: f32,
    pub paragraph_gap_ratio: f32,
    pub merge_lines: bool,
}

impl Default for ParagraphGroupingOptions {
    fn default() -> Self {
        Self {
            y_tolerance: 0.015,
            paragraph_gap_ratio: 1.8,
            merge_lines: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OcrParagraph {
    pub lines: Vec<RecognizedText>,
    pub text: String,
    pub bbox: BBox,
    pub confidence: f32,
}

pub fn sort_text_regions_reading_order(
    mut items: Vec<RecognizedText>,
    _page_width: f32,
    page_height: f32,
) -> Vec<RecognizedText> {
    let y_tolerance = (page_height.max(1.0) * 0.015).max(1.0);
    items.sort_by(|a, b| {
        let a_y = (a.region.bbox.y0 + a.region.bbox.y1) / 2.0;
        let b_y = (b.region.bbox.y0 + b.region.bbox.y1) / 2.0;
        let dy = (a_y - b_y).abs();
        if dy <= y_tolerance {
            a.region
                .bbox
                .x0
                .partial_cmp(&b.region.bbox.x0)
                .unwrap_or(std::cmp::Ordering::Equal)
        } else {
            a_y.partial_cmp(&b_y).unwrap_or(std::cmp::Ordering::Equal)
        }
    });
    items
}

pub fn group_ocr_lines_into_paragraphs(
    lines: Vec<RecognizedText>,
    options: ParagraphGroupingOptions,
) -> Vec<OcrParagraph> {
    if lines.is_empty() {
        return vec![];
    }

    let mut paragraphs: Vec<OcrParagraph> = Vec::new();
    let mut current: Vec<RecognizedText> = Vec::new();

    for line in lines {
        if current.is_empty() {
            current.push(line);
            continue;
        }

        let prev = current.last().expect("line exists");
        let prev_h = (prev.region.bbox.y1 - prev.region.bbox.y0).max(1.0);
        let gap = line.region.bbox.y0 - prev.region.bbox.y1;
        let near = gap <= prev_h * options.paragraph_gap_ratio + options.y_tolerance;

        if options.merge_lines && near {
            current.push(line);
        } else {
            paragraphs.push(paragraph_from_lines(std::mem::take(&mut current)));
            current.push(line);
        }
    }

    if !current.is_empty() {
        paragraphs.push(paragraph_from_lines(current));
    }

    paragraphs
}

pub fn filter_low_confidence(
    items: Vec<RecognizedText>,
    min_text_confidence: f32,
    drop_low_confidence: bool,
) -> (Vec<RecognizedText>, Vec<RecognizedText>) {
    let mut kept = Vec::new();
    let mut low = Vec::new();

    for item in items {
        if item.confidence < min_text_confidence {
            if !drop_low_confidence {
                kept.push(item.clone());
            }
            low.push(item);
        } else {
            kept.push(item);
        }
    }

    (kept, low)
}

fn paragraph_from_lines(lines: Vec<RecognizedText>) -> OcrParagraph {
    let mut x0 = f32::MAX;
    let mut y0 = f32::MAX;
    let mut x1 = 0.0_f32;
    let mut y1 = 0.0_f32;
    let mut confidence = 0.0_f32;

    for line in &lines {
        x0 = x0.min(line.region.bbox.x0);
        y0 = y0.min(line.region.bbox.y0);
        x1 = x1.max(line.region.bbox.x1);
        y1 = y1.max(line.region.bbox.y1);
        confidence += line.confidence;
    }

    let text = lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let avg_conf = if lines.is_empty() {
        0.0
    } else {
        confidence / lines.len() as f32
    };

    OcrParagraph {
        lines,
        text,
        bbox: BBox { x0, y0, x1, y1 },
        confidence: avg_conf,
    }
}
