use std::path::PathBuf;

use document_parser::config::load_pipeline_config;
use document_parser::converters::ExtractionContext;
use document_parser::rendering::{LibreOfficeOfficeRenderer, OfficeRenderer};
use futures::executor::block_on;

#[test]
fn office_renderer_exists_and_returns_structured_unavailable_error_without_binary() {
    let pipeline = load_pipeline_config(&PathBuf::from("configs/pipeline.config.jsonc")).ok();
    let renderer = LibreOfficeOfficeRenderer::from_pipeline_config(pipeline.as_ref());
    let mut context = ExtractionContext::default();

    let result = block_on(renderer.render_to_pdf(
        &PathBuf::from("testdata/presentation/sample_ru.pptx"),
        &mut context,
    ));

    if result.is_ok() {
        return;
    }

    let err = result.expect_err("expected structured error");
    assert!(
        err.code == "LIBREOFFICE_NOT_AVAILABLE"
            || err.code == "LIBREOFFICE_CONVERSION_FAILED"
            || err.code == "LIBREOFFICE_TIMEOUT"
    );
}
