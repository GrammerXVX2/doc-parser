use std::env;
use std::path::PathBuf;

use anyhow::Context;
use document_parser::api::run_server;
use document_parser::config::{load_format_routing_config, load_pipeline_config};
use document_parser::config::profiles::ServiceProfile;
use document_parser::doctor::{DoctorOptions, render_doctor_report_text, run_doctor};
use document_parser::observability::init_tracing;
use document_parser::performance::run_benchmark;
use document_parser::pipeline::{PipelineContext, run_pipeline};
use document_parser::quality::{generate_quality_report_from_model_path, write_quality_report};
use document_parser::runtime::{
    OcrCliOverrides, PipelineCliOverrides, set_ocr_cli_overrides, set_output_root_dir,
    set_pipeline_cli_overrides,
};
use document_parser::writer::write_document_outputs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let mut args = env::args().skip(1);
    let command = args
        .next()
        .context("usage: document_parser <parse|serve|bench|quality|doctor> ...")?;

    if command == "serve" {
        let mut config_path = PathBuf::from("configs/profiles/api.jsonc");
        while let Some(arg) = args.next() {
            if arg == "--config" {
                if let Some(path) = args.next() {
                    config_path = PathBuf::from(path);
                }
            }
        }

        let profile = ServiceProfile::from_path(&config_path)?;
        run_server(profile).await?;
        return Ok(());
    }

    if command == "bench" {
        let mut input_dir = PathBuf::from("testdata/benchmark");
        let mut output_dir = PathBuf::from("data/bench_output");
        let mut profile_path = PathBuf::from("configs/profiles/benchmark.jsonc");

        while let Some(arg) = args.next() {
            if arg == "--input" {
                if let Some(path) = args.next() {
                    input_dir = PathBuf::from(path);
                }
            } else if arg == "--output" {
                if let Some(path) = args.next() {
                    output_dir = PathBuf::from(path);
                }
            } else if arg == "--profile" {
                if let Some(path) = args.next() {
                    profile_path = PathBuf::from(path);
                }
            }
        }

        let pipeline_config = load_pipeline_config(&profile_path)?;
        let routing_config =
            load_format_routing_config(PathBuf::from("configs/format_routing.config.jsonc").as_path())?;
        let report = run_benchmark(&input_dir, &output_dir, pipeline_config, routing_config)?;

        println!(
            "Benchmark done: documents={} pages={} duration_ms={} docs_per_sec={:.2} pages_per_sec={:.2}",
            report.documents,
            report.pages,
            report.duration_ms,
            report.documents_per_second,
            report.pages_per_second
        );
        return Ok(());
    }

    if command == "quality" {
        let mut input = None::<PathBuf>;
        let mut output_dir = None::<PathBuf>;
        while let Some(arg) = args.next() {
            if arg == "--input" {
                input = args.next().map(PathBuf::from);
            } else if arg == "--output" {
                output_dir = args.next().map(PathBuf::from);
            }
        }

        let input = input.context("usage: document_parser quality --input <model.json> [--output <dir>]" )?;
        let report = generate_quality_report_from_model_path(&input)?;
        let default_out = input
            .parent()
            .map(|v| v.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let output_dir = output_dir.unwrap_or(default_out);
        let (json_path, md_path) = write_quality_report(&report, &output_dir)?;

        println!(
            "Quality report done: json={} md={}",
            json_path.display(),
            md_path.display()
        );
        return Ok(());
    }

    if command == "doctor" {
        let mut options = DoctorOptions::default();
        let mut as_json = false;

        while let Some(arg) = args.next() {
            if arg == "--pipeline-config" {
                if let Some(path) = args.next() {
                    options.pipeline_config_path = PathBuf::from(path);
                }
            } else if arg == "--service-profile" {
                if let Some(path) = args.next() {
                    options.service_profile_path = PathBuf::from(path);
                }
            } else if arg == "--json" {
                as_json = true;
            }
        }

        let report = run_doctor(&options);
        if as_json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            println!("{}", render_doctor_report_text(&report));
        }
        return Ok(());
    }

    if command != "parse" {
        anyhow::bail!(
            "unknown command '{command}', expected 'parse', 'serve', 'bench', 'quality' or 'doctor'"
        );
    }

    let input = args
        .next()
        .map(PathBuf::from)
        .context("usage: document_parser parse <input_path> --output <output_dir>")?;

    let mut output_dir = PathBuf::from("output");
    let mut ocr_overrides = OcrCliOverrides::default();
    let mut pipeline_overrides = PipelineCliOverrides::default();
    while let Some(arg) = args.next() {
        if arg == "--output" {
            if let Some(dir) = args.next() {
                output_dir = PathBuf::from(dir);
            }
        } else if arg == "--ocr-backend" {
            ocr_overrides.backend = args.next();
        } else if arg == "--ocr-det-model" {
            ocr_overrides.det_model = args.next();
        } else if arg == "--ocr-rec-model" {
            ocr_overrides.rec_model = args.next();
        } else if arg == "--ocr-charset" {
            ocr_overrides.charset = args.next();
        } else if arg == "--ocr-provider" {
            ocr_overrides.provider = args.next();
        } else if arg == "--ml-provider" {
            ocr_overrides.provider = args.next();
        } else if arg == "--triton-url" {
            ocr_overrides.triton_url = args.next();
        } else if arg == "--save-crops" {
            ocr_overrides.save_crops = args
                .next()
                .as_deref()
                .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes"));
        } else if arg == "--language" {
            pipeline_overrides.language = args.next();
        } else if arg == "--languages" {
            pipeline_overrides.languages = args.next().map(|v| {
                v.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            });
        } else if arg == "--locale" {
            pipeline_overrides.locale = args.next();
        } else if arg == "--normalize-ru" {
            pipeline_overrides.normalize_ru = args.next();
        } else if arg == "--extract-tables" {
            pipeline_overrides.extract_tables = args
                .next()
                .as_deref()
                .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes"));
        } else if arg == "--table-chunks" {
            pipeline_overrides.table_chunks = args
                .next()
                .as_deref()
                .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes"));
        } else if arg == "--layout-backend" {
            pipeline_overrides.layout_backend = args.next();
        } else if arg == "--detect-scanned-tables" {
            pipeline_overrides.detect_scanned_tables = args
                .next()
                .as_deref()
                .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes"));
        } else if arg == "--detect-formulas" {
            pipeline_overrides.detect_formulas = args
                .next()
                .as_deref()
                .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes"));
        } else if arg == "--debug-layout" {
            pipeline_overrides.debug_layout = args
                .next()
                .as_deref()
                .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes"));
        } else if arg == "--reading-order" {
            pipeline_overrides.reading_order = args.next();
        } else if arg == "--exclude-headers-footers-from-chunks" {
            pipeline_overrides.exclude_headers_footers_from_chunks = args
                .next()
                .as_deref()
                .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes"));
        }
    }

    set_output_root_dir(output_dir.clone());
    set_ocr_cli_overrides(ocr_overrides);
    set_pipeline_cli_overrides(pipeline_overrides);

    let pipeline_config = load_pipeline_config(PathBuf::from("configs/pipeline.config.jsonc").as_path())?;
    let routing_config =
        load_format_routing_config(PathBuf::from("configs/format_routing.config.jsonc").as_path())?;
    let context = PipelineContext::new(pipeline_config, routing_config);

    let (classification, mut model) = run_pipeline(&input, &context)?;

    let pretty = context
        .pipeline_config
        .pipeline
        .output
        .get("pretty_json")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    model.processing.stages.push(document_parser::model::ProcessingStage {
        name: "output_write".to_string(),
        status: document_parser::model::StageStatus::Ok,
        tool: "output_writer".to_string(),
        duration_ms: Some(1),
        metadata: serde_json::json!({}),
    });

    let output_file = write_document_outputs(&model, &output_dir, pretty)?;

    println!(
        "Parsed {} as {:?} using {} elements -> {}",
        input.display(),
        classification.likely_format,
        model.stats.element_count,
        output_file.display()
    );

    Ok(())
}
