use std::path::PathBuf;

use document_parser::classifier::classify_file;
use document_parser::extractors::docx::DocxExtractor;
use document_parser::router::Extractor;
use document_parser::validation::{ValidationSeverity, validate_document_model};

#[test]
fn docx_extractor_extracts_stage5_elements() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("testdata/office/sample_ru.docx");
    let classification = classify_file(&path).expect("classification");

    let model = DocxExtractor
        .extract(&path, &classification)
        .expect("docx extraction");

    let elements = &model.pages[0].elements;
    assert!(elements.iter().any(|e| matches!(e.element_type, document_parser::model::ElementType::Heading)));
    assert!(elements.iter().any(|e| matches!(e.element_type, document_parser::model::ElementType::List)));
    assert!(elements.iter().any(|e| matches!(e.element_type, document_parser::model::ElementType::Table)));
    assert!(elements.iter().any(|e| matches!(e.element_type, document_parser::model::ElementType::Image)));

    let roles = elements
        .iter()
        .filter_map(|e| e.role.as_deref())
        .collect::<Vec<_>>();
    assert!(roles.iter().any(|r| *r == "footnote"));
    assert!(roles.iter().any(|r| *r == "endnote"));
    assert!(roles.iter().any(|r| *r == "comment"));
    assert!(roles.iter().any(|r| *r == "header"));
    assert!(roles.iter().any(|r| *r == "footer"));

    assert!(model.assets.iter().any(|a| a.asset_type == "embedded_image"));

    let issues = validate_document_model(&model);
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.severity, ValidationSeverity::Fatal)),
        "fatal validation issues: {:?}",
        issues
    );
}
