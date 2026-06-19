use std::path::Path;

use async_trait::async_trait;
use serde_json::Value;

use crate::config::{PipelineConfig, pipeline_converters_value};
use crate::converters::traits::{
    ConversionError, ConversionStageRecord, ConversionTarget, ConvertedDocument, DocumentConverter,
    ExtractionContext, Result,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TikaMode {
    Disabled,
    Server,
    Jar,
}

#[derive(Debug, Clone)]
pub struct TikaConverter {
    pub enabled: bool,
    pub mode: TikaMode,
}

impl Default for TikaConverter {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: TikaMode::Disabled,
        }
    }
}

impl TikaConverter {
    pub fn from_pipeline_config(config: Option<&PipelineConfig>) -> Self {
        let mut out = Self::default();

        let Some(config) = config else {
            return out;
        };
        let tika = pipeline_converters_value(config, "tika").unwrap_or(&Value::Null);

        out.enabled = tika.get("enabled").and_then(Value::as_bool).unwrap_or(false);
        out.mode = match tika
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("disabled")
            .to_ascii_lowercase()
            .as_str()
        {
            "server" => TikaMode::Server,
            "jar" => TikaMode::Jar,
            "server_or_jar" => TikaMode::Server,
            _ => TikaMode::Disabled,
        };

        out
    }
}

#[async_trait]
impl DocumentConverter for TikaConverter {
    fn name(&self) -> &'static str {
        "tika"
    }

    fn supports_conversion(&self, _input_path: &Path, target: ConversionTarget) -> bool {
        matches!(target, ConversionTarget::Text | ConversionTarget::Html)
    }

    async fn convert(
        &self,
        _input_path: &Path,
        target: ConversionTarget,
        context: &mut ExtractionContext,
    ) -> Result<ConvertedDocument> {
        context.push_stage(ConversionStageRecord::warning("external_command_start", self.name()));

        if !self.enabled || self.mode == TikaMode::Disabled {
            return Err(ConversionError::new(
                "TIKA_DISABLED",
                "Apache Tika отключен в конфигурации. Резервная конвертация недоступна.",
            ));
        }

        let _ = target;
        Err(ConversionError::new(
            "TIKA_NOT_CONFIGURED",
            "Apache Tika пока не сконфигурирован для текущего режима. Требуется дополнительная настройка.",
        ))
    }
}
