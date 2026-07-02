use std::path::PathBuf;

use document_parser::classifier::{DetectedFormat, FileClassification};
use document_parser::extractors::base_document_model;
use document_parser::models::config::load_model_stack_config;
use document_parser::models::router::route_models;
use document_parser::model::{ContentMode, DocumentFormat, PageType};

fn classification_pdf() -> FileClassification {
    FileClassification {
        input_path: PathBuf::from("sample.pdf"),
        extension: "pdf".to_string(),
        mime_by_extension: Some("application/pdf".to_string()),
        mime_by_magic: Some("application/pdf".to_string()),
        size_bytes: 10,
        sha256: "x".to_string(),
        likely_format: DetectedFormat::Pdf,
        is_encrypted_or_protected: false,
    }
}

fn model_with_text(text: &str) -> document_parser::model::DocumentModel {
    let cls = classification_pdf();
    let mut model = base_document_model(&cls, DocumentFormat::Pdf, ContentMode::Digital, PageType::DocumentPage);
    model.pages[0].text = text.to_string();
    model
}

#[test]
fn legal_routes_to_legal_fast() {
    let cfg = load_model_stack_config(&PathBuf::from("configs/model_stack.config.jsonc")).unwrap();
    let cls = classification_pdf();
    let model = model_with_text("Договор, Заказчик, ИНН 7701234567");

    let decision = route_models(&cls, Some(&model), &cfg, None, None);
    assert_eq!(decision.selected_profile, "legal_fast");
}

#[test]
fn legal_high_accuracy_override_is_used() {
    let cfg = load_model_stack_config(&PathBuf::from("configs/model_stack.config.jsonc")).unwrap();
    let cls = classification_pdf();
    let model = model_with_text("Договор, Заказчик, ИНН 7701234567");

    let decision = route_models(&cls, Some(&model), &cfg, Some("legal_high_accuracy"), None);
    assert_eq!(decision.selected_profile, "legal_high_accuracy");
}

#[test]
fn fiction_routes_to_fiction_modern() {
    let cfg = load_model_stack_config(&PathBuf::from("configs/model_stack.config.jsonc")).unwrap();
    let cls = classification_pdf();
    let model = model_with_text("Глава 1. Роман.");

    let decision = route_models(&cls, Some(&model), &cfg, None, None);
    assert_eq!(decision.selected_profile, "fiction_modern");
}

#[test]
fn historical_markers_route_to_fiction_historical() {
    let cfg = load_model_stack_config(&PathBuf::from("configs/model_stack.config.jsonc")).unwrap();
    let cls = classification_pdf();
    let model = model_with_text("Текст с ѣ и і в дореформенной орфографии.");

    let decision = route_models(&cls, Some(&model), &cfg, None, None);
    assert_eq!(decision.selected_profile, "fiction_historical");
}

#[test]
fn scientific_routes_to_scientific_profile() {
    let cfg = load_model_stack_config(&PathBuf::from("configs/model_stack.config.jsonc")).unwrap();
    let cls = classification_pdf();
    let model = model_with_text("Теорема. Лемма. Формула.");

    let decision = route_models(&cls, Some(&model), &cfg, None, None);
    assert_eq!(decision.selected_profile, "scientific");
}

#[test]
fn unknown_routes_to_mixed_enterprise() {
    let cfg = load_model_stack_config(&PathBuf::from("configs/model_stack.config.jsonc")).unwrap();
    let cls = classification_pdf();
    let model = model_with_text("Нейтральный текст без специальных маркеров.");

    let decision = route_models(&cls, Some(&model), &cfg, None, None);
    assert_eq!(decision.selected_profile, "mixed_enterprise");
}
