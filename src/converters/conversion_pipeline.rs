use std::path::Path;

use crate::converters::traits::{
    ConversionError, ConversionStageRecord, ConversionTarget, ConvertedDocument, DocumentConverter,
    ExtractionContext, Result,
};

pub struct ConversionPipeline {
    pub converters: Vec<Box<dyn DocumentConverter>>,
}

impl Default for ConversionPipeline {
    fn default() -> Self {
        Self { converters: vec![] }
    }
}

impl ConversionPipeline {
    pub fn new(converters: Vec<Box<dyn DocumentConverter>>) -> Self {
        Self { converters }
    }

    pub async fn convert_with_fallbacks(
        &self,
        input_path: &Path,
        targets: &[ConversionTarget],
        context: &mut ExtractionContext,
    ) -> Result<ConvertedDocument> {
        let mut failures = Vec::new();

        for target in targets {
            for converter in &self.converters {
                if !converter.supports_conversion(input_path, *target) {
                    continue;
                }

                match converter.convert(input_path, *target, context).await {
                    Ok(result) => return Ok(result),
                    Err(err) => {
                        context.push_warning(err.clone());
                        context.push_stage(
                            ConversionStageRecord::warning("conversion_attempt_failed", converter.name())
                                .with_meta("code", err.code.clone())
                                .with_meta("target", target.as_str()),
                        );
                        failures.push(err);
                    }
                }
            }
        }

        if failures.is_empty() {
            return Err(ConversionError::new(
                "CONVERTER_NOT_CONFIGURED",
                "Для данного документа не найден ни один подходящий конвертер.",
            ));
        }

        let summary = failures
            .iter()
            .map(|e| format!("{}: {}", e.code, e.message))
            .collect::<Vec<_>>()
            .join("; ");

        Err(ConversionError::new(
            "CONVERTER_NOT_CONFIGURED",
            format!("Все попытки fallback-конвертации завершились ошибкой: {}", summary),
        ))
    }
}
