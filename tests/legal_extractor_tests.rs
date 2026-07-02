use std::path::PathBuf;

use document_parser::classifier::{DetectedFormat, FileClassification};
use document_parser::extractors::base_document_model;
use document_parser::models::legal::extract_legal_mvp;
use document_parser::model::{ContentMode, DocumentFormat, PageType};

fn model_with_legal_text(text: &str) -> document_parser::model::DocumentModel {
    let classification = FileClassification {
        input_path: PathBuf::from("contract.txt"),
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
fn extracts_identifiers_dates_amounts_and_parties() {
    let text = r#"
    Договор № 45-АБ
    Заказчик: ООО Ромашка
    Исполнитель: ООО Василек
    ИНН: 7701234567
    КПП: 770101001
    ОГРН: 1027700132195
    от 12.03.2024
    Цена: 1 250 000 руб.
    В случае просрочки применяется неустойка.
    "#;

    let model = model_with_legal_text(text);
    let legal = extract_legal_mvp(&model);

    assert!(!legal.identifiers.is_empty());
    assert!(legal.identifiers.iter().any(|v| v.kind == "ИНН"));
    assert!(legal.identifiers.iter().any(|v| v.kind == "КПП"));
    assert!(legal.identifiers.iter().any(|v| v.kind == "ОГРН"));
    assert!(!legal.dates.is_empty());
    assert!(!legal.amounts.is_empty());
    assert!(legal.identifiers.iter().any(|v| v.kind == "contract_number"));
    assert!(legal.parties.len() >= 2);
}
