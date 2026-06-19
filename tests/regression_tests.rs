use std::path::PathBuf;

use document_parser::regression::{
    discover_cases, run_case, run_regression_suite,
};

#[test]
fn corpus_cases_discovered() {
    let root = PathBuf::from("regression/corpus");
    let cases = discover_cases(&root).unwrap();
    assert!(!cases.is_empty());
}

#[test]
fn run_single_case_with_assertions() {
    let case = PathBuf::from("regression/corpus/html/ru_html_basic/case.jsonc");
    let expected = PathBuf::from("regression/expected");

    let result = run_case(
        &case,
        &expected,
        &PathBuf::from("configs/pipeline.config.jsonc"),
        &PathBuf::from("configs/format_routing.config.jsonc"),
    )
    .unwrap();

    assert_eq!(result.case_id, "ru_html_basic");
}

#[test]
fn run_suite_executes_all_cases() {
    let summary = run_regression_suite(
        &PathBuf::from("regression/corpus"),
        &PathBuf::from("regression/expected"),
        &PathBuf::from("configs/pipeline.config.jsonc"),
        &PathBuf::from("configs/format_routing.config.jsonc"),
    )
    .unwrap();

    assert!(summary.total > 0);
    assert_eq!(summary.total, summary.passed + summary.failed);
}
