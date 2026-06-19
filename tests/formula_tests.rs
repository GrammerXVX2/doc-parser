use std::fs;

use futures::executor::block_on;

use document_parser::classifier::classify_file;
use document_parser::converters::traits::ExtractionContext;
use document_parser::extractors::html::HtmlExtractor;
use document_parser::extractors::markdown::MarkdownExtractor;
use document_parser::formulas::{
    FixtureFormulaDetector, FormulaDetectionInput, FormulaDetector, create_formula_placeholder,
};
use document_parser::router::Extractor;

#[test]
fn markdown_block_formula_is_extracted() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("testdata/ru/formulas_ru.md");
    let classification = classify_file(&path).expect("classification");
    let model = MarkdownExtractor
        .extract(&path, &classification)
        .expect("extract");

    assert!(model.pages[0]
        .elements
        .iter()
        .any(|e| matches!(e.element_type, document_parser::model::ElementType::Formula)));
}

#[test]
fn html_mathml_formula_is_extracted() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("target/test_formula_mathml.html");
    let _ = fs::create_dir_all(root.join("target"));
    fs::write(
        &path,
        "<html><body><math><mrow><mi>x</mi><mo>=</mo><mn>1</mn></mrow></math></body></html>",
    )
    .expect("write");

    let classification = classify_file(&path).expect("classification");
    let model = HtmlExtractor.extract(&path, &classification).expect("extract");
    assert!(model.pages[0]
        .elements
        .iter()
        .any(|e| matches!(e.element_type, document_parser::model::ElementType::Formula)));
}

#[test]
fn fixture_formula_region_creates_placeholder() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input = FormulaDetectionInput {
        document_id: "doc".to_string(),
        page_number: 1,
        page_image_path: Some(root.join("testdata/images/formula_scan.png")),
        page_width: 1000.0,
        page_height: 1400.0,
    };
    let detector = FixtureFormulaDetector;
    let mut ctx = ExtractionContext::default();
    let regions = block_on(detector.detect_formulas(input, &mut ctx)).expect("fixture works");

    let placeholder = create_formula_placeholder(
        regions[0].page_number,
        &regions[0].region_id,
        regions[0].bbox,
        regions[0].confidence,
        &regions[0].source,
    );
    let text = placeholder
        .content
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(text.contains("Формула обнаружена"));
    assert_eq!(
        placeholder
            .extra
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or_default(),
        "unknown"
    );
}
