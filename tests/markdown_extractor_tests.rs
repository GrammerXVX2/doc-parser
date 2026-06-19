use std::path::PathBuf;

use document_parser::classifier::classify_file;
use document_parser::extractors::markdown::MarkdownExtractor;
use document_parser::router::Extractor;

#[test]
fn markdown_extractor_extracts_expected_elements() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("testdata/sample.md");
    let classification = classify_file(&path).expect("classification should work");

    let extractor = MarkdownExtractor;
    let model = extractor
        .extract(&path, &classification)
        .expect("markdown extraction should succeed");

    let text = &model.pages[0].text;
    assert!(text.contains("Sample Header"));
    assert!(text.contains("Paragraph line."));
    assert!(text.contains("one"));
    assert!(text.contains("quote"));
    assert!(text.contains("fn main()"));
    assert!(model.document_profile.has_images);
    assert!(
        model.pages[0]
            .elements
            .iter()
            .any(|e| matches!(e.element_type, document_parser::model::ElementType::Table))
    );
}
