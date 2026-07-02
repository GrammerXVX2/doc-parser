use std::path::PathBuf;

use document_parser::classifier::{DetectedFormat, FileClassification};
use document_parser::extractors::base_document_model;
use document_parser::models::domain::{DocumentDomain, detect_document_domain};
use document_parser::model::{ContentMode, DocumentFormat, PageType};

fn model_with_text(text: &str) -> document_parser::model::DocumentModel {
    let classification = FileClassification {
        input_path: PathBuf::from("sample.txt"),
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
fn legal_markers_detect_legal_domain() {
    let model = model_with_text("Договор поставки. ИНН 7701234567. Заказчик и Исполнитель.");
    let profile = detect_document_domain(&model);
    assert_eq!(profile.domain, DocumentDomain::Legal);
}

#[test]
fn fiction_markers_detect_fiction_domain() {
    let model = model_with_text("Глава 1. Роман в письмах. Часть первая.");
    let profile = detect_document_domain(&model);
    assert_eq!(profile.domain, DocumentDomain::Fiction);
}

#[test]
fn historical_markers_detect_historical_domain() {
    let model = model_with_text("Старая орѳографія: міръ и ѣсть.");
    let profile = detect_document_domain(&model);
    assert_eq!(profile.domain, DocumentDomain::HistoricalBook);
}

#[test]
fn scientific_markers_detect_scientific_domain() {
    let model = model_with_text("Теорема. Доказательство. Формула и уравнение. DOI 10.1000/test.");
    let profile = detect_document_domain(&model);
    assert_eq!(profile.domain, DocumentDomain::Scientific);
}

#[test]
fn mixed_stats_detect_mixed_enterprise() {
    let mut model = model_with_text("Нейтральный текст без маркеров");
    model.stats.table_count = 1;
    model.stats.image_count = 1;
    model.stats.formula_count = 1;
    model.stats.text_element_count = 2;

    let profile = detect_document_domain(&model);
    assert_eq!(profile.domain, DocumentDomain::MixedEnterprise);
}
