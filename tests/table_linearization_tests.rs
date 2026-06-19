use document_parser::tables::{TableCell, TableLinearizationOptions, linearize_cells};

#[test]
fn russian_linearization_and_chunk_limits() {
    let mut cells = Vec::new();
    let headers = ["Метрика", "Значение", "Комментарий"];
    for (c, h) in headers.iter().enumerate() {
        cells.push(TableCell {
            row: 0,
            column: c,
            rowspan: 1,
            colspan: 1,
            bbox: None,
            text: (*h).to_string(),
            html: None,
            markdown: None,
            formula: None,
            is_header: true,
            confidence: None,
        });
    }

    for r in 1..5 {
        cells.push(TableCell {
            row: r,
            column: 0,
            rowspan: 1,
            colspan: 1,
            bbox: None,
            text: format!("Метрика {}", r),
            html: None,
            markdown: None,
            formula: None,
            is_header: false,
            confidence: None,
        });
        cells.push(TableCell {
            row: r,
            column: 1,
            rowspan: 1,
            colspan: 1,
            bbox: None,
            text: format!("{}", r * 100),
            html: None,
            markdown: None,
            formula: None,
            is_header: false,
            confidence: None,
        });
        cells.push(TableCell {
            row: r,
            column: 2,
            rowspan: 1,
            colspan: 1,
            bbox: None,
            text: "январь".to_string(),
            html: None,
            markdown: None,
            formula: None,
            is_header: false,
            confidence: None,
        });
    }

    let chunks = linearize_cells(
        &cells,
        5,
        3,
        TableLinearizationOptions {
            max_rows_per_chunk: 2,
            language: "ru".to_string(),
        },
    );

    assert!(chunks.len() >= 2);
    assert!(chunks[0].text.contains("Строка"));
    assert!(chunks[0].text.contains("Метрика"));
}
