use std::path::PathBuf;

use document_parser::classifier::classify_file;
use document_parser::extractors::markdown::MarkdownExtractor;
use document_parser::router::Extractor;

#[test]
fn markdown_pipe_table_extraction_and_russian_preservation() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("testdata/ru/table_ru.md");
    let classification = classify_file(&path).expect("classification");

    let model = MarkdownExtractor
        .extract(&path, &classification)
        .expect("markdown extraction");

    let table = model.pages[0]
        .elements
        .iter()
        .find(|e| matches!(e.element_type, document_parser::model::ElementType::Table))
        .expect("table element expected");

    let md = table.content.get("markdown").and_then(|v| v.as_str()).unwrap_or("");
    assert!(md.contains("Метрика"));
    assert!(table.content.get("csv").is_some());
    assert!(table.content.get("html").is_some());
}
