use std::path::PathBuf;

use document_parser::classifier::classify_file;
use document_parser::extractors::html::HtmlExtractor;
use document_parser::router::Extractor;

#[test]
fn html_extractor_extracts_expected_elements() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("testdata/sample.html");
    let classification = classify_file(&path).expect("classification should work");

    let extractor = HtmlExtractor;
    let model = extractor
        .extract(&path, &classification)
        .expect("html extraction should succeed");

    let types = model.pages[0]
        .elements
        .iter()
        .map(|e| format!("{:?}", e.element_type))
        .collect::<Vec<_>>();

    assert!(types.iter().any(|t| t == "Heading"));
    assert!(types.iter().any(|t| t == "Paragraph"));
    assert!(types.iter().any(|t| t == "List" || t == "ListItem"));
    assert!(types.iter().any(|t| t == "Code"));
    assert!(types.iter().any(|t| t == "Blockquote"));
    assert!(model.document_profile.has_tables);
    assert!(types.iter().any(|t| t == "Table"));
}
