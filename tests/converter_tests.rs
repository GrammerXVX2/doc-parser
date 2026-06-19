use std::path::PathBuf;

use document_parser::config::load_pipeline_config;
use document_parser::converters::{
    ConversionPipeline, ConversionTarget, DocumentConverter, ExtractionContext, LibreOfficeConverter,
    PandocConverter, TikaConverter,
};
use document_parser::utils::command_exists::command_exists;
use futures::executor::block_on;

#[test]
fn libreoffice_unavailable_returns_structured_error_no_panic() {
    let mut converter = LibreOfficeConverter::default();
    converter.binary = "definitely_missing_soffice".to_string();
    let mut context = ExtractionContext::default();

    let err = block_on(converter.convert(
        &PathBuf::from("testdata/legacy/sample_ru.doc"),
        ConversionTarget::Docx,
        &mut context,
    ))
    .expect_err("expected unavailable");

    assert_eq!(err.code, "LIBREOFFICE_NOT_AVAILABLE");
}

#[test]
fn pandoc_unavailable_returns_structured_error_no_panic() {
    let mut converter = PandocConverter::default();
    converter.binary = "definitely_missing_pandoc".to_string();
    let mut context = ExtractionContext::default();

    let err = block_on(converter.convert(
        &PathBuf::from("testdata/legacy/sample_ru.rtf"),
        ConversionTarget::Html,
        &mut context,
    ))
    .expect_err("expected unavailable");

    assert_eq!(err.code, "PANDOC_NOT_AVAILABLE");
}

#[test]
fn conversion_pipeline_collects_failures() {
    let mut libre = LibreOfficeConverter::default();
    libre.binary = "definitely_missing_soffice".to_string();
    let mut pandoc = PandocConverter::default();
    pandoc.binary = "definitely_missing_pandoc".to_string();

    let pipeline = ConversionPipeline::new(vec![Box::new(pandoc), Box::new(libre), Box::new(TikaConverter::default())]);
    let mut context = ExtractionContext::default();

    let err = block_on(pipeline.convert_with_fallbacks(
        &PathBuf::from("testdata/legacy/sample_ru.rtf"),
        &[ConversionTarget::Html, ConversionTarget::Docx, ConversionTarget::Pdf, ConversionTarget::Text],
        &mut context,
    ))
    .expect_err("expected fallback failure");

    assert_eq!(err.code, "CONVERTER_NOT_CONFIGURED");
    assert!(!context.warnings.is_empty());
}

#[test]
fn optional_external_converter_integration_when_enabled() {
    if std::env::var("RUN_EXTERNAL_CONVERTER_TESTS").unwrap_or_default() != "1" {
        return;
    }

    if command_exists("pandoc") {
        let mut ctx = ExtractionContext::default();
        let out = block_on(PandocConverter::default().convert(
            &PathBuf::from("testdata/legacy/sample_ru.rtf"),
            ConversionTarget::Html,
            &mut ctx,
        ));
        assert!(out.is_ok(), "pandoc conversion should work when binary exists");
    }

    if command_exists("soffice") {
        let mut ctx = ExtractionContext::default();
        let out = block_on(LibreOfficeConverter::default().convert(
            &PathBuf::from("testdata/legacy/sample_ru.doc"),
            ConversionTarget::Docx,
            &mut ctx,
        ));
        assert!(out.is_ok(), "libreoffice conversion should work when binary exists");
    }

    let _ = load_pipeline_config(&PathBuf::from("configs/pipeline.config.jsonc"));
}
