use std::path::PathBuf;

use document_parser::classifier::classify_file;
use document_parser::extractors::txt::TxtExtractor;
use document_parser::router::Extractor;

#[test]
fn txt_pipe_and_tsv_table_extraction() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("testdata/ru/table_ru.txt");
    let classification = classify_file(&path).expect("classification");

    let model = TxtExtractor
        .extract(&path, &classification)
        .expect("txt extraction");

    let table_count = model.pages[0]
        .elements
        .iter()
        .filter(|e| matches!(e.element_type, document_parser::model::ElementType::Table))
        .count();
    assert!(table_count >= 2);
    assert!(
        model.pages[0]
            .elements
            .iter()
            .any(|e| matches!(e.element_type, document_parser::model::ElementType::Paragraph))
    );
}
