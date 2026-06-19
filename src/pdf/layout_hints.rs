use crate::pdf::spans::PdfTextBlock;

pub fn infer_block_role_hint(block: &PdfTextBlock, median_font_size: Option<f32>) -> Option<String> {
    let text = block.text.trim();
    if text.is_empty() {
        return None;
    }

    let near_top = block.bbox.y0 <= 180.0;
    let short = text.chars().count() <= 120;

    if let Some(median) = median_font_size {
        let font_values = block
            .lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .filter_map(|s| s.font_size)
            .collect::<Vec<_>>();
        if !font_values.is_empty() {
            let avg = font_values.iter().sum::<f32>() / font_values.len() as f32;
            let bold = block
                .lines
                .iter()
                .flat_map(|l| l.spans.iter())
                .any(|s| s.bold.unwrap_or(false));
            if avg >= median * 1.2 && short && (bold || near_top) {
                return Some("section_title".to_string());
            }
        }
    }

    None
}
