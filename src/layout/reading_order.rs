use crate::layout::types::LayoutRegion;
use crate::model::{Element, ElementType, Page};

#[derive(Debug, Clone)]
pub struct ReadingOrderOptions {
    pub strategy: String,
    pub multi_column: bool,
    pub header_footer_handling: String,
    pub y_tolerance: f32,
    pub column_gap_threshold: f32,
}

impl Default for ReadingOrderOptions {
    fn default() -> Self {
        Self {
            strategy: "layout_aware".to_string(),
            multi_column: true,
            header_footer_handling: "mark".to_string(),
            y_tolerance: 0.02,
            column_gap_threshold: 0.08,
        }
    }
}

pub fn assign_layout_aware_reading_order(
    pages: &mut [Page],
    _layout_regions: &[LayoutRegion],
    options: ReadingOrderOptions,
) -> anyhow::Result<()> {
    if options.strategy.eq_ignore_ascii_case("natural") {
        let mut global = 1_u32;
        for page in pages {
            for (idx, element) in page.elements.iter_mut().enumerate() {
                element.reading_order = Some((idx + 1) as u32);
                element.global_order = Some(global);
                global += 1;
            }
        }
        return Ok(());
    }

    let mut global = 1_u32;
    for page in pages {
        let page_width = page.width.unwrap_or(1000.0);
        let mut indexed = page
            .elements
            .iter()
            .enumerate()
            .map(|(idx, el)| {
                let bbox = el.bbox.unwrap_or([0.0, idx as f32 * 20.0, page_width, idx as f32 * 20.0 + 20.0]);
                (idx, bbox)
            })
            .collect::<Vec<_>>();

        let split_x = page_width * 0.5;
        let mut left_count = 0usize;
        let mut right_count = 0usize;
        let mut min_right_x = f32::MAX;
        let mut max_left_x: f32 = 0.0;

        for (idx, bbox) in &indexed {
            let element = &page.elements[*idx];
            if !is_text_like(element) {
                continue;
            }
            let cx = (bbox[0] + bbox[2]) / 2.0;
            if cx < split_x {
                left_count += 1;
                max_left_x = max_left_x.max(bbox[2]);
            } else {
                right_count += 1;
                min_right_x = min_right_x.min(bbox[0]);
            }
        }

        let has_two_columns = options.multi_column
            && left_count >= 2
            && right_count >= 2
            && min_right_x.is_finite()
            && (min_right_x - max_left_x) >= page_width * options.column_gap_threshold;

        indexed.sort_by(|(idx_a, bbox_a), (idx_b, bbox_b)| {
            let elem_a = &page.elements[*idx_a];
            let elem_b = &page.elements[*idx_b];

            let col_a = if has_two_columns && is_text_like(elem_a) {
                if (bbox_a[0] + bbox_a[2]) / 2.0 < split_x {
                    0
                } else {
                    1
                }
            } else {
                0
            };
            let col_b = if has_two_columns && is_text_like(elem_b) {
                if (bbox_b[0] + bbox_b[2]) / 2.0 < split_x {
                    0
                } else {
                    1
                }
            } else {
                0
            };

            col_a
                .cmp(&col_b)
                .then_with(|| bbox_a[1].partial_cmp(&bbox_b[1]).unwrap_or(std::cmp::Ordering::Equal))
                .then_with(|| bbox_a[0].partial_cmp(&bbox_b[0]).unwrap_or(std::cmp::Ordering::Equal))
        });

        // Captions should follow figure/table where close enough.
        let mut ordered_indices = indexed.into_iter().map(|(idx, _)| idx).collect::<Vec<_>>();
        for cap_pos in 0..ordered_indices.len() {
            let cap_idx = ordered_indices[cap_pos];
            if !is_caption(&page.elements[cap_idx]) {
                continue;
            }

            let Some(cap_bbox) = page.elements[cap_idx].bbox else {
                continue;
            };
            let mut best_anchor_pos = None;
            let mut best_distance = f32::MAX;
            for (pos, idx) in ordered_indices.iter().enumerate() {
                if pos >= cap_pos {
                    break;
                }
                let anchor = &page.elements[*idx];
                if !matches!(anchor.element_type, ElementType::Image | ElementType::Table | ElementType::Chart) {
                    continue;
                }
                let Some(anchor_bbox) = anchor.bbox else {
                    continue;
                };
                let dy = (cap_bbox[1] - anchor_bbox[3]).abs();
                let x_overlap = overlap_1d(cap_bbox[0], cap_bbox[2], anchor_bbox[0], anchor_bbox[2]);
                if x_overlap > 0.0 && dy < best_distance {
                    best_distance = dy;
                    best_anchor_pos = Some(pos);
                }
            }

            if let Some(anchor_pos) = best_anchor_pos {
                if anchor_pos + 1 != cap_pos && best_distance <= page.height.unwrap_or(1400.0) * 0.1 {
                    let moved = ordered_indices.remove(cap_pos);
                    ordered_indices.insert(anchor_pos + 1, moved);
                }
            }
        }

        let mut reordered = Vec::with_capacity(page.elements.len());
        for idx in ordered_indices {
            reordered.push(page.elements[idx].clone());
        }

        for (idx, element) in reordered.iter_mut().enumerate() {
            if options.header_footer_handling == "exclude_from_chunks"
                && matches!(element.element_type, ElementType::Header | ElementType::Footer)
            {
                element
                    .extra
                    .insert("exclude_from_chunks".to_string(), serde_json::json!(true));
            }
            element.reading_order = Some((idx + 1) as u32);
            element.global_order = Some(global);
            global += 1;
        }

        page.elements = reordered;
    }

    Ok(())
}

fn is_text_like(element: &Element) -> bool {
    matches!(
        element.element_type,
        ElementType::Text
            | ElementType::TextOcr
            | ElementType::Heading
            | ElementType::Paragraph
            | ElementType::List
            | ElementType::ListItem
            | ElementType::Blockquote
            | ElementType::Code
            | ElementType::Caption
            | ElementType::Header
            | ElementType::Footer
    )
}

fn is_caption(element: &Element) -> bool {
    matches!(element.element_type, ElementType::Caption)
        || element
            .role
            .as_deref()
            .map(|r| r.contains("caption"))
            .unwrap_or(false)
}

fn overlap_1d(a0: f32, a1: f32, b0: f32, b1: f32) -> f32 {
    (a1.min(b1) - a0.max(b0)).max(0.0)
}
