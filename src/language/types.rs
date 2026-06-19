use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub primary: Option<String>,
    #[serde(default)]
    pub detected: Vec<DetectedLanguage>,
    #[serde(default)]
    pub hints: Vec<String>,
    pub is_mixed: bool,
    pub confidence: Option<f32>,
}

impl Default for LanguageInfo {
    fn default() -> Self {
        Self {
            primary: Some("ru".to_string()),
            detected: vec![DetectedLanguage {
                language: "ru".to_string(),
                confidence: 0.2,
                source: LanguageDetectionSource::ConfigDefault,
            }],
            hints: vec!["ru".to_string(), "en".to_string()],
            is_mixed: false,
            confidence: Some(0.2),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedLanguage {
    pub language: String,
    pub confidence: f32,
    pub source: LanguageDetectionSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageDetectionSource {
    ConfigDefault,
    UserHint,
    FileMetadata,
    TextDetection,
    OcrModel,
    Extractor,
}
