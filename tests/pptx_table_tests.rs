use std::path::PathBuf;

use document_parser::classifier::classify_file;
use document_parser::extractors::pptx::PptxExtractor;
use document_parser::model::ElementType;
use document_parser::router::Extractor;

#[test]
fn pptx_table_uses_unified_table_model_and_chunks() {
    let path = PathBuf::from("testdata/presentation/sample_tables.pptx");
    let classification = classify_file(&path).expect("classification");

    let model = PptxExtractor.extract(&path, &classification).expect("extract");
    let table = model
        .pages
        .iter()
        .flat_map(|p| p.elements.iter())
        .find(|e| matches!(e.element_type, ElementType::Table))
        .expect("table expected");

    assert!(table.extra.get("cells").is_some());
    assert!(table.content.get("markdown").and_then(|v| v.as_str()).unwrap_or_default().contains("|"));
    assert!(table.content.get("csv").and_then(|v| v.as_str()).unwrap_or_default().contains(","));
    assert!(table.content.get("html").and_then(|v| v.as_str()).unwrap_or_default().contains("<table"));
    assert!(table.extra.get("linearized_chunks").is_some());
}
