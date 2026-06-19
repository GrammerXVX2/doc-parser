use std::fs;
use std::path::PathBuf;

use document_parser::config::{load_format_routing_config, load_pipeline_config};
use document_parser::model::DocumentFormat;
use document_parser::pipeline::{PipelineContext, run_pipeline};

fn test_dir() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "document_parser_tests_{}_{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("smoke")
    ));
    let _ = fs::create_dir_all(&root);
    root
}

fn pipeline_context() -> PipelineContext {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pipeline = load_pipeline_config(&root.join("configs/pipeline.config.jsonc"))
        .expect("pipeline config should load");
    let routing = load_format_routing_config(&root.join("configs/format_routing.config.jsonc"))
        .expect("routing config should load");
    PipelineContext::new(pipeline, routing)
}

#[test]
fn html_file_is_extracted() {
    let dir = test_dir();
    let input = dir.join("sample.html");
    fs::write(
        &input,
        "<html><body><h1>Title</h1><p>Hello world.</p><ul><li>One</li></ul></body></html>",
    )
    .expect("write html fixture");

    let (_, model) = run_pipeline(&input, &pipeline_context()).expect("pipeline should succeed for html");

    assert_eq!(model.document_profile.format, DocumentFormat::Html);
    assert!(model.stats.element_count > 0);
    assert!(model.pages[0].text.contains("Title"));
}

#[test]
fn markdown_file_is_extracted() {
    let dir = test_dir();
    let input = dir.join("sample.md");
    fs::write(&input, "# Header\n\n- one\n- two\n\n```\nlet x = 1;\n```")
        .expect("write markdown fixture");

    let (_, model) = run_pipeline(&input, &pipeline_context()).expect("pipeline should succeed for md");

    assert_eq!(model.document_profile.format, DocumentFormat::Md);
    assert!(model.stats.element_count >= 3);
    assert!(model.pages[0].markdown.contains("# Header"));
}

#[test]
fn txt_file_is_extracted() {
    let dir = test_dir();
    let input = dir.join("sample.txt");
    fs::write(&input, "TITLE\n\n- item\nplain text")
        .expect("write text fixture");

    let (_, model) = run_pipeline(&input, &pipeline_context()).expect("pipeline should succeed for txt");

    assert_eq!(model.document_profile.format, DocumentFormat::Txt);
    assert!(model.stats.element_count >= 2);
    assert!(model.pages[0].text.contains("plain text"));
}
