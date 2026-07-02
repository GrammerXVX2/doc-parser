use std::path::PathBuf;

use document_parser::classifier::{DetectedFormat, FileClassification};
use document_parser::extractors::base_document_model;
use document_parser::models::books::extract_book_mvp;
use document_parser::model::{ContentMode, DocumentFormat, PageType};

fn model_with_book_text(text: &str) -> document_parser::model::DocumentModel {
    let classification = FileClassification {
        input_path: PathBuf::from("book.txt"),
        extension: "txt".to_string(),
        mime_by_extension: Some("text/plain".to_string()),
        mime_by_magic: Some("text/plain".to_string()),
        size_bytes: text.len() as u64,
        sha256: "x".to_string(),
        likely_format: DetectedFormat::Txt,
        is_encrypted_or_protected: false,
    };
    let mut model = base_document_model(
        &classification,
        DocumentFormat::Txt,
        ContentMode::PlainText,
        PageType::SyntheticTextPage,
    );
    model.pages[0].text = text.to_string();
    model
}

#[test]
fn detects_chapters_footnotes_dehyphenation_and_historical_markers() {
    let text = r#"
    Глава 1. Начало
    Это сло-
    во должно склеиться.
    1. Сноска номер один
    Текст с символом ѣ.
    "#;

    let model = model_with_book_text(text);
    let book = extract_book_mvp(&model);

    assert!(!book.chapters.is_empty());
    assert!(!book.footnotes.is_empty());
    assert!(book.dehyphenation_applied);
    assert!(book.historical_orthography_detected);
}
