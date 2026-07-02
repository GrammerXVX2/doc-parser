use std::path::PathBuf;

use document_parser::config::{load_format_routing_config, load_pipeline_config};
use document_parser::pipeline::{PipelineContext, run_pipeline};

#[test]
fn model_outputs_and_domain_profile_are_written() {
    let pipeline = load_pipeline_config(PathBuf::from("configs/pipeline.config.jsonc").as_path()).unwrap();
    let routing = load_format_routing_config(PathBuf::from("configs/format_routing.config.jsonc").as_path()).unwrap();
    let context = PipelineContext::new(pipeline, routing);

    let input = PathBuf::from("testdata/ru/sample_ru.html");
    let (_, model) = run_pipeline(&input, &context).unwrap();

    assert!(model.extra.contains_key("model_outputs"));
    assert!(model.extra.contains_key("domain_profile"));
}

#[test]
fn legal_document_adds_legal_output() {
    let dir = std::env::temp_dir().join(format!("legal_model_outputs_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("legal_sample.txt");
    std::fs::write(&input, "Договор. Заказчик. Исполнитель. ИНН 7701234567. КПП 770101001. 12.03.2024").unwrap();

    let pipeline = load_pipeline_config(PathBuf::from("configs/pipeline.config.jsonc").as_path()).unwrap();
    let routing = load_format_routing_config(PathBuf::from("configs/format_routing.config.jsonc").as_path()).unwrap();
    let context = PipelineContext::new(pipeline, routing);

    let (_, model) = run_pipeline(&input, &context).unwrap();
    assert!(model.extra.contains_key("legal"));
}

#[test]
fn book_document_adds_book_output() {
    let dir = std::env::temp_dir().join(format!("book_model_outputs_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("book_sample.txt");
    std::fs::write(&input, "Глава 1. Роман. Это сло-\nво и сноска\n1. Примечание").unwrap();

    let pipeline = load_pipeline_config(PathBuf::from("configs/pipeline.config.jsonc").as_path()).unwrap();
    let routing = load_format_routing_config(PathBuf::from("configs/format_routing.config.jsonc").as_path()).unwrap();
    let context = PipelineContext::new(pipeline, routing);

    let (_, model) = run_pipeline(&input, &context).unwrap();
    assert!(model.extra.contains_key("book"));
}
