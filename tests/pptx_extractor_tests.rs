use std::path::PathBuf;

use document_parser::classifier::classify_file;
use document_parser::extractors::pptx::PptxExtractor;
use document_parser::model::{ElementType, PageType};
use document_parser::router::Extractor;
use document_parser::validation::{ValidationSeverity, validate_document_model};

#[test]
fn pptx_extractor_extracts_slides_titles_lists_tables_images_notes() {
    let path = PathBuf::from("testdata/presentation/sample_ru.pptx");
    let classification = classify_file(&path).expect("classification");

    let model = PptxExtractor.extract(&path, &classification).expect("extract");

    assert!(!model.pages.is_empty());
    assert!(matches!(model.pages[0].page_type, PageType::Slide));

    let elements = model.pages.iter().flat_map(|p| p.elements.iter()).collect::<Vec<_>>();
    assert!(elements.iter().any(|e| matches!(e.element_type, ElementType::Heading)));
    assert!(elements.iter().any(|e| matches!(e.element_type, ElementType::Text)));
    assert!(elements.iter().any(|e| matches!(e.element_type, ElementType::List)));
    assert!(elements.iter().any(|e| matches!(e.element_type, ElementType::Table)));
    assert!(elements.iter().any(|e| matches!(e.element_type, ElementType::Image)));
    assert!(elements.iter().any(|e| e.role.as_deref() == Some("speaker_notes")));

    assert_eq!(model.document_profile.languages.first().map(|v| v.as_str()), Some("ru"));

    let issues = validate_document_model(&model);
    assert!(
        !issues.iter().any(|i| matches!(i.severity, ValidationSeverity::Fatal)),
        "fatal validation issues: {:?}",
        issues
    );
}
