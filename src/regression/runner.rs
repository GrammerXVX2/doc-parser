use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{load_format_routing_config, load_pipeline_config};
use crate::pipeline::{PipelineContext, run_pipeline};
use crate::regression::assertions::{
    RegressionExpectation, RegressionTolerance, evaluate_assertions,
};
use crate::regression::normalize::normalize_model_json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionCaseConfig {
    pub case_id: String,
    pub format: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub expected: RegressionExpectation,
    #[serde(default)]
    pub tolerances: RegressionTolerance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionCaseResult {
    pub case_id: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegressionRunSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<RegressionCaseResult>,
}

pub fn discover_cases(corpus_root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !corpus_root.exists() {
        return Ok(out);
    }

    let mut stack = vec![corpus_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(path);
                continue;
            }

            if path
                .file_name()
                .and_then(|v| v.to_str())
                .map(|v| v.eq_ignore_ascii_case("case.jsonc"))
                .unwrap_or(false)
            {
                out.push(path);
            }
        }
    }

    out.sort();
    Ok(out)
}

pub fn run_regression_suite(
    corpus_root: &Path,
    expected_root: &Path,
    pipeline_config_path: &Path,
    routing_config_path: &Path,
) -> anyhow::Result<RegressionRunSummary> {
    let cases = discover_cases(corpus_root)?;
    let mut summary = RegressionRunSummary {
        total: cases.len(),
        ..RegressionRunSummary::default()
    };

    for case_path in cases {
        match run_case(
            &case_path,
            expected_root,
            pipeline_config_path,
            routing_config_path,
        ) {
            Ok(result) => {
                if result.ok {
                    summary.passed = summary.passed.saturating_add(1);
                } else {
                    summary.failed = summary.failed.saturating_add(1);
                }
                summary.results.push(result);
            }
            Err(error) => {
                summary.failed = summary.failed.saturating_add(1);
                summary.results.push(RegressionCaseResult {
                    case_id: case_path.display().to_string(),
                    ok: false,
                    message: format!("REGRESSION_ASSERTION_FAILED: {}", error),
                });
            }
        }
    }

    Ok(summary)
}

pub fn run_case(
    case_config_path: &Path,
    expected_root: &Path,
    pipeline_config_path: &Path,
    routing_config_path: &Path,
) -> anyhow::Result<RegressionCaseResult> {
    let raw = std::fs::read_to_string(case_config_path)?;
    let config: RegressionCaseConfig = json5::from_str(&raw)?;

    let case_dir = case_config_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid regression case path"))?;
    let input = detect_case_input(case_dir)?;

    let pipeline = load_pipeline_config(pipeline_config_path)?;
    let routing = load_format_routing_config(routing_config_path)?;
    let context = PipelineContext::new(pipeline, routing);

    let (_classification, model) = run_pipeline(&input, &context)?;
    evaluate_assertions(&config.expected.assertions, &model)?;

    let current = serde_json::to_value(&model)?;
    let normalized = normalize_model_json(&current, &config.tolerances.ignore_fields);

    let expected_file = expected_root
        .join(&config.format)
        .join(format!("{}.model.json", config.case_id));

    if expected_file.exists() {
        let expected_raw = std::fs::read_to_string(&expected_file)?;
        let expected_json: Value = serde_json::from_str(&expected_raw)?;
        let normalized_expected = normalize_model_json(&expected_json, &config.tolerances.ignore_fields);
        if normalized != normalized_expected {
            return Ok(RegressionCaseResult {
                case_id: config.case_id,
                ok: false,
                message: format!(
                    "GOLDEN_SNAPSHOT_MISMATCH: snapshot не совпадает ({})",
                    expected_file.display()
                ),
            });
        }
    }

    Ok(RegressionCaseResult {
        case_id: config.case_id,
        ok: true,
        message: "ok".to_string(),
    })
}

fn detect_case_input(case_dir: &Path) -> anyhow::Result<PathBuf> {
    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(case_dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
            continue;
        };
        if name.eq_ignore_ascii_case("case.jsonc") || name.eq_ignore_ascii_case("README.md") {
            continue;
        }
        candidates.push(path);
    }

    candidates.sort();
    candidates
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("REGRESSION_ASSERTION_FAILED: не найден input файл кейса"))
}

fn default_language() -> String {
    "ru".to_string()
}
