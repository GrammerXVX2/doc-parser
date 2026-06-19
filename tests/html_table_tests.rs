use std::path::PathBuf;

use document_parser::classifier::classify_file;
use document_parser::extractors::html::HtmlExtractor;
use document_parser::router::Extractor;

#[test]
fn html_table_extraction_unified_model() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("testdata/ru/table_ru.html");
    let classification = classify_file(&path).expect("classification");

    let model = HtmlExtractor
        .extract(&path, &classification)
        .expect("html extraction");

    let table = model.pages[0]
        .elements
        .iter()
        .find(|e| matches!(e.element_type, document_parser::model::ElementType::Table))
        .expect("table element expected");

    assert!(table.extra.get("rows").and_then(|v| v.as_u64()).unwrap_or(0) >= 2);
    assert!(table.extra.get("columns").and_then(|v| v.as_u64()).unwrap_or(0) >= 2);
    assert!(table.content.get("markdown").and_then(|v| v.as_str()).unwrap_or("").contains("|"));
    assert!(table.content.get("csv").and_then(|v| v.as_str()).unwrap_or("").contains(","));
    assert!(table.content.get("html").and_then(|v| v.as_str()).unwrap_or("").contains("<table>"));
    assert!(table.extra.get("cells").is_some());
}
