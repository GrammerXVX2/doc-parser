use std::path::PathBuf;

use document_parser::classifier::classify_file;
use document_parser::extractors::docx::DocxExtractor;
use document_parser::router::Extractor;

#[test]
fn docx_table_uses_unified_table_model_and_ru_linearization() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("testdata/office/sample_tables.docx");
    let classification = classify_file(&path).expect("classification");

    let model = DocxExtractor
        .extract(&path, &classification)
        .expect("docx extraction");

    let table = model.pages[0]
        .elements
        .iter()
        .find(|e| matches!(e.element_type, document_parser::model::ElementType::Table))
        .expect("table element");

    let rows = table.extra.get("rows").and_then(|v| v.as_u64()).unwrap_or(0);
    let cols = table.extra.get("columns").and_then(|v| v.as_u64()).unwrap_or(0);
    assert!(rows >= 2);
    assert!(cols >= 2);

    assert!(table.content.get("markdown").and_then(|v| v.as_str()).unwrap_or("").contains("|"));
    assert!(table.content.get("csv").and_then(|v| v.as_str()).unwrap_or("").contains(","));
    assert!(table.content.get("html").and_then(|v| v.as_str()).unwrap_or("").contains("<table>"));
    assert!(table.extra.get("cells").is_some());

    let linearized = table
        .extra
        .get("linearized_chunks")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(!linearized.is_empty());
    let text = linearized[0].get("text").and_then(|v| v.as_str()).unwrap_or("");
    assert!(text.contains("Строка"));
}
