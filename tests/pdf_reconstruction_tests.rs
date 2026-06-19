use document_parser::model::ElementType;
use document_parser::pdf::{
    PdfTextReconstructionOptions, PdfTextSpan, merge_lines_into_blocks, merge_spans_into_lines,
    pdf_blocks_to_elements,
};
use document_parser::utils::geometry::BBox;

#[test]
fn spans_merge_into_lines_and_blocks() {
    let spans = vec![
        PdfTextSpan {
            text: "Заголовок".to_string(),
            bbox: BBox {
                x0: 40.0,
                y0: 40.0,
                x1: 220.0,
                y1: 58.0,
            },
            font_size: Some(20.0),
            font_name: Some("Times".to_string()),
            bold: Some(true),
            italic: Some(false),
        },
        PdfTextSpan {
            text: "Текст".to_string(),
            bbox: BBox {
                x0: 40.0,
                y0: 90.0,
                x1: 120.0,
                y1: 106.0,
            },
            font_size: Some(12.0),
            font_name: Some("Times".to_string()),
            bold: Some(false),
            italic: Some(false),
        },
        PdfTextSpan {
            text: "абзаца".to_string(),
            bbox: BBox {
                x0: 130.0,
                y0: 90.0,
                x1: 220.0,
                y1: 106.0,
            },
            font_size: Some(12.0),
            font_name: Some("Times".to_string()),
            bold: Some(false),
            italic: Some(false),
        },
    ];

    let options = PdfTextReconstructionOptions::default();
    let lines = merge_spans_into_lines(spans, options.clone());
    assert!(lines.len() >= 2);

    let blocks = merge_lines_into_blocks(lines, options);
    assert!(!blocks.is_empty());

    let elements = pdf_blocks_to_elements(blocks);
    assert!(!elements.is_empty());
    assert!(
        elements
            .iter()
            .any(|e| matches!(e.element_type, ElementType::Heading))
    );
}

#[test]
fn russian_text_is_normalized_for_paragraphs() {
    let block = document_parser::pdf::PdfTextBlock {
        lines: vec![],
        text: "  Пример\tТекста  ".to_string(),
        bbox: BBox {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 20.0,
        },
        role_hint: None,
    };

    let elements = pdf_blocks_to_elements(vec![block]);
    let text = elements[0]
        .content
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(text.contains("Пример"));
    assert!(!text.contains("\t"));
}
