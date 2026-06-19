use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::config::{FormatRoutingConfig, PipelineConfig};
use crate::performance::latency::LatencyTracker;
use crate::pipeline::{PipelineContext, run_pipeline};
use crate::writer::write_document_outputs;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BenchmarkLatencyReport {
    pub p50: u64,
    pub p95: u64,
    pub p99: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BenchmarkOcrReport {
    pub backend: String,
    pub recognition_batches: u64,
    pub avg_recognition_batch_size: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BenchmarkReport {
    pub documents: usize,
    pub pages: usize,
    pub duration_ms: u64,
    pub documents_per_second: f64,
    pub pages_per_second: f64,
    pub latency_ms: BenchmarkLatencyReport,
    pub ocr: BenchmarkOcrReport,
}

pub fn run_benchmark(
    input_dir: &Path,
    output_dir: &Path,
    pipeline_config: PipelineConfig,
    routing_config: FormatRoutingConfig,
) -> anyhow::Result<BenchmarkReport> {
    if !input_dir.exists() {
        return Err(anyhow::anyhow!(
            "BENCHMARK_FAILED: input directory does not exist: {}",
            input_dir.display()
        ));
    }

    std::fs::create_dir_all(output_dir)?;

    let context = PipelineContext::new(pipeline_config.clone(), routing_config);
    let mut files = collect_files(input_dir)?;
    files.sort();

    let started = Instant::now();
    let latency = LatencyTracker::new();

    let mut pages = 0_usize;
    let mut processed_documents = 0_usize;
    let mut recognition_batches = 0_u64;
    let mut recognition_items = 0_u64;

    for path in &files {
        let doc_started = Instant::now();
        let (_classification, model) = match run_pipeline(path, &context) {
            Ok(result) => result,
            Err(error) => {
                tracing::warn!(
                    code = "BENCHMARK_FAILED",
                    file = %path.display(),
                    "Benchmark skip: failed to process input file: {}",
                    error
                );
                continue;
            }
        };
        processed_documents = processed_documents.saturating_add(1);
        let elapsed_ms = doc_started.elapsed().as_millis() as f64;
        latency.observe(elapsed_ms);

        pages = pages.saturating_add(model.pages.len());

        for page in &model.pages {
            for element in &page.elements {
                let Some(metrics) = element
                    .extra
                    .get("ocr_metrics")
                    .and_then(|v| v.get("counters"))
                else {
                    continue;
                };

                if let Some(v) = metrics.get("ocr_recognition_batches").and_then(|v| v.as_u64()) {
                    recognition_batches = recognition_batches.saturating_add(v);
                }
                if let Some(v) = metrics.get("ocr_recognized_regions").and_then(|v| v.as_u64()) {
                    recognition_items = recognition_items.saturating_add(v);
                }
            }
        }

        write_document_outputs(&model, output_dir, true)?;
    }

    if processed_documents == 0 {
        return Err(anyhow::anyhow!(
            "BENCHMARK_FAILED: no supported documents were processed"
        ));
    }

    let duration_ms = started.elapsed().as_millis() as u64;
    let seconds = (duration_ms as f64 / 1000.0).max(0.001);
    let summary = latency.summary();

    let report = BenchmarkReport {
        documents: processed_documents,
        pages,
        duration_ms,
        documents_per_second: processed_documents as f64 / seconds,
        pages_per_second: pages as f64 / seconds,
        latency_ms: BenchmarkLatencyReport {
            p50: summary.p50_ms.round() as u64,
            p95: summary.p95_ms.round() as u64,
            p99: summary.p99_ms.round() as u64,
        },
        ocr: BenchmarkOcrReport {
            backend: pipeline_config
                .pipeline
                .ocr
                .get("backend")
                .and_then(|v| v.as_str())
                .unwrap_or("mock")
                .to_string(),
            recognition_batches,
            avg_recognition_batch_size: if recognition_batches == 0 {
                0.0
            } else {
                recognition_items as f64 / recognition_batches as f64
            },
        },
    };

    write_report(output_dir, &report)?;
    Ok(report)
}

fn write_report(output_dir: &Path, report: &BenchmarkReport) -> anyhow::Result<()> {
    let json_path = output_dir.join("bench_report.json");
    let md_path = output_dir.join("bench_report.md");

    std::fs::write(&json_path, serde_json::to_vec_pretty(report)?)?;

    let mut md = String::new();
    let _ = writeln!(md, "# Benchmark Report");
    let _ = writeln!(md);
    let _ = writeln!(md, "- Documents: {}", report.documents);
    let _ = writeln!(md, "- Pages: {}", report.pages);
    let _ = writeln!(md, "- Duration (ms): {}", report.duration_ms);
    let _ = writeln!(md, "- Documents/sec: {:.2}", report.documents_per_second);
    let _ = writeln!(md, "- Pages/sec: {:.2}", report.pages_per_second);
    let _ = writeln!(md);
    let _ = writeln!(md, "## Latency");
    let _ = writeln!(md, "- p50: {} ms", report.latency_ms.p50);
    let _ = writeln!(md, "- p95: {} ms", report.latency_ms.p95);
    let _ = writeln!(md, "- p99: {} ms", report.latency_ms.p99);
    let _ = writeln!(md);
    let _ = writeln!(md, "## OCR");
    let _ = writeln!(md, "- Backend: {}", report.ocr.backend);
    let _ = writeln!(md, "- Recognition batches: {}", report.ocr.recognition_batches);
    let _ = writeln!(
        md,
        "- Avg recognition batch size: {:.2}",
        report.ocr.avg_recognition_batch_size
    );

    std::fs::write(&md_path, md)?;
    Ok(())
}

fn collect_files(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(path) = stack.pop() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(path);
            } else {
                files.push(path);
            }
        }
    }

    Ok(files)
}
