use std::collections::HashMap;
use std::path::Path;

use crate::classifier::FileClassification;
use crate::config::PipelineConfig;
use crate::converters::{
    ConversionError, ConversionPipeline, ConversionTarget, ExtractionContext, LibreOfficeConverter,
    PandocConverter, TikaConverter,
};
use crate::extractors::{
    base_document_model, stage, update_stats,
};
use crate::extractors::docx::DocxExtractor;
use crate::extractors::html::HtmlExtractor;
use crate::extractors::pdf::PdfExtractor;
use crate::model::{
    ContentMode, Diagnostic, DocumentFormat, ProcessingStage, ProcessingStatus, StageStatus,
};
use crate::router::Extractor;

use futures::executor::block_on;

#[derive(Default)]
pub struct RtfExtractor;

impl Extractor for RtfExtractor {
    fn name(&self) -> &'static str {
        "rtf_extractor"
    }

    fn extract(
        &self,
        input_path: &Path,
        classification: &FileClassification,
    ) -> anyhow::Result<crate::model::DocumentModel> {
        let mut fallback_model = base_document_model(
            classification,
            DocumentFormat::Rtf,
            ContentMode::Digital,
            crate::model::PageType::DocumentPage,
        );
        fallback_model.coordinate_system.unit = "synthetic".to_string();
        fallback_model.processing.stages.push(stage(
            "rtf_conversion_pipeline",
            "conversion_pipeline",
            1,
        ));

        let pipeline_cfg = load_pipeline_config();
        let pipeline = ConversionPipeline::new(vec![
            Box::new(PandocConverter::from_pipeline_config(pipeline_cfg.as_ref())),
            Box::new(LibreOfficeConverter::from_pipeline_config(pipeline_cfg.as_ref())),
            Box::new(TikaConverter::from_pipeline_config(pipeline_cfg.as_ref())),
        ]);

        let mut context = ExtractionContext::default();
        let conversion = block_on(pipeline.convert_with_fallbacks(
            input_path,
            &[
                ConversionTarget::Html,
                ConversionTarget::Docx,
                ConversionTarget::Pdf,
                ConversionTarget::Text,
            ],
            &mut context,
        ));

        let converted = match conversion {
            Ok(v) => v,
            Err(err) => {
                fallback_model.errors.push(to_diagnostic(
                    if err.code == "CONVERTER_NOT_CONFIGURED" {
                        "RTF_NO_FALLBACK_AVAILABLE"
                    } else {
                        "RTF_CONVERSION_FAILED"
                    },
                    &err,
                    "document",
                    "error",
                ));
                fallback_model.processing.status = ProcessingStatus::Failed;
                append_context_stages(&mut fallback_model, &context, "rtf");
                fallback_model
                    .processing
                    .stages
                    .push(stage("rtf_route_converted_document", "router", 0));
                fallback_model.processing.total_duration_ms =
                    Some(fallback_model.processing.stages.len() as u64);
                update_stats(&mut fallback_model);
                return Ok(fallback_model);
            }
        };

        let converted_class = crate::classifier::classify_file(&converted.path)?;
        let mut routed = match converted.target {
            ConversionTarget::Html => HtmlExtractor.extract(&converted.path, &converted_class)?,
            ConversionTarget::Docx => DocxExtractor.extract(&converted.path, &converted_class)?,
            ConversionTarget::Pdf => PdfExtractor.extract(&converted.path, &converted_class)?,
            _ => {
                let mut text_model = base_document_model(
                    classification,
                    DocumentFormat::Rtf,
                    ContentMode::Digital,
                    crate::model::PageType::DocumentPage,
                );
                let text = std::fs::read_to_string(&converted.path).unwrap_or_default();
                text_model.pages[0].text = text.clone();
                text_model.pages[0].markdown = text;
                text_model
            }
        };

        routed.source = fallback_model.source.clone();
        routed.document_profile.format = DocumentFormat::Rtf;
        routed.processing.stages.insert(
            0,
            stage("rtf_conversion_pipeline", "conversion_pipeline", 1),
        );
        routed.processing.stages.push(stage(
            match converted.target {
                ConversionTarget::Html => "rtf_convert_to_html",
                ConversionTarget::Docx => "rtf_convert_to_docx",
                ConversionTarget::Pdf => "rtf_convert_to_pdf",
                ConversionTarget::Text => "rtf_convert_to_text",
                ConversionTarget::Markdown => "rtf_convert_to_markdown",
            },
            &converted.converter_name,
            converted.duration_ms,
        ));
        routed
            .processing
            .stages
            .push(stage("rtf_route_converted_document", "router", 1));

        append_context_stages(&mut routed, &context, "rtf");

        for page in &mut routed.pages {
            for element in &mut page.elements {
                element.provenance = serde_json::json!({
                    "method": "converted",
                    "tool": format!("{}+{}", converted.converter_name, tool_suffix_for_target(converted.target)),
                    "stage": match converted.target {
                        ConversionTarget::Html => "rtf_to_html_extraction",
                        ConversionTarget::Docx => "rtf_to_docx_extraction",
                        ConversionTarget::Pdf => "rtf_to_pdf_extraction",
                        ConversionTarget::Text => "rtf_to_text_extraction",
                        ConversionTarget::Markdown => "rtf_to_markdown_extraction",
                    },
                    "source_ref": {
                        "kind": "converted_file",
                        "value": converted.path.to_string_lossy(),
                    }
                });
            }
        }

        routed.processing.total_duration_ms = Some(routed.processing.stages.len() as u64);
        update_stats(&mut routed);
        Ok(routed)
    }
}

fn load_pipeline_config() -> Option<PipelineConfig> {
    crate::config::load_pipeline_config(Path::new("configs/pipeline.config.jsonc")).ok()
}

fn to_diagnostic(code: &str, error: &ConversionError, scope: &str, severity: &str) -> Diagnostic {
    let mut extra = HashMap::new();
    for (k, v) in &error.metadata {
        extra.insert(k.clone(), serde_json::json!(v));
    }

    Diagnostic {
        code: code.to_string(),
        severity: severity.to_string(),
        scope: scope.to_string(),
        page_number: None,
        element_id: None,
        message: error.message.clone(),
        recoverable: error.recoverable,
        extra,
    }
}

fn append_context_stages(model: &mut crate::model::DocumentModel, context: &ExtractionContext, prefix: &str) {
    for stage_rec in &context.stage_records {
        model.processing.stages.push(ProcessingStage {
            name: format!("{}_{}", prefix, stage_rec.name),
            status: match stage_rec.status.as_str() {
                "ok" => StageStatus::Ok,
                "warning" => StageStatus::Warning,
                "error" => StageStatus::Error,
                _ => StageStatus::Warning,
            },
            tool: stage_rec.tool.clone(),
            duration_ms: Some(0),
            metadata: serde_json::to_value(&stage_rec.metadata).unwrap_or_else(|_| serde_json::json!({})),
        });
    }

    for warning in &context.warnings {
        model.warnings.push(to_diagnostic(&warning.code, warning, "document", "warning"));
    }
}

fn tool_suffix_for_target(target: ConversionTarget) -> &'static str {
    match target {
        ConversionTarget::Docx => "docx_ooxml_parser",
        ConversionTarget::Pdf => "pdf_native_extractor",
        ConversionTarget::Html => "html_dom_parser",
        ConversionTarget::Markdown => "markdown_parser",
        ConversionTarget::Text => "text_parser",
    }
}
