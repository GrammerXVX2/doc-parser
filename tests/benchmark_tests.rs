use std::path::PathBuf;

use document_parser::config::{load_format_routing_config, load_pipeline_config};
use document_parser::performance::run_benchmark;

#[test]
fn benchmark_report_generated() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input_dir = std::env::temp_dir().join(format!(
        "doc_parser_bench_input_{}",
        uuid::Uuid::new_v4()
    ));
    let output_dir = std::env::temp_dir().join(format!(
        "doc_parser_bench_output_{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&input_dir).unwrap();
    std::fs::write(input_dir.join("sample.txt"), "Привет benchmark").unwrap();

    let pipeline = load_pipeline_config(&root.join("configs/profiles/benchmark.jsonc")).unwrap();
    let routing = load_format_routing_config(&root.join("configs/format_routing.config.jsonc")).unwrap();

    let report = run_benchmark(&input_dir, &output_dir, pipeline, routing).unwrap();

    assert_eq!(report.documents, 1);
    assert!(report.pages_per_second > 0.0);
    assert!(output_dir.join("bench_report.json").exists());
    assert!(output_dir.join("bench_report.md").exists());
}
