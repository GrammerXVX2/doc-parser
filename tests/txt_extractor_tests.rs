use std::path::PathBuf;

use document_parser::classifier::classify_file;
use document_parser::extractors::txt::TxtExtractor;
use document_parser::router::Extractor;

#[test]
fn txt_extractor_builds_synthetic_page_and_paragraphs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("testdata/sample.txt");
    let classification = classify_file(&path).expect("classification should work");

    let extractor = TxtExtractor;
    let model = extractor
        .extract(&path, &classification)
        .expect("txt extraction should succeed");

    assert_eq!(model.pages.len(), 1);
    assert!(!model.pages[0].elements.is_empty());
    assert!(model.pages[0].text.contains("This is a paragraph."));
    assert!(
        model.pages[0]
            .elements
            .iter()
            .any(|e| matches!(e.element_type, document_parser::model::ElementType::Table))
    );
}
