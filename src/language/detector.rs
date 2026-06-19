use crate::language::types::{DetectedLanguage, LanguageDetectionSource, LanguageInfo};

pub trait LanguageDetector {
    fn detect(&self, text: &str, hints: &[String]) -> LanguageInfo;
}

#[derive(Debug, Clone)]
pub struct HeuristicLanguageDetector {
    pub default_language: String,
    pub detect_min_chars: usize,
}

impl Default for HeuristicLanguageDetector {
    fn default() -> Self {
        Self {
            default_language: "ru".to_string(),
            detect_min_chars: 40,
        }
    }
}

impl LanguageDetector for HeuristicLanguageDetector {
    fn detect(&self, text: &str, hints: &[String]) -> LanguageInfo {
        let trimmed = text.trim();
        let hint_vec = if hints.is_empty() {
            vec![self.default_language.clone(), "en".to_string()]
        } else {
            hints.to_vec()
        };

        if trimmed.is_empty() || trimmed.chars().count() < self.detect_min_chars {
            return LanguageInfo {
                primary: Some(self.default_language.clone()),
                detected: vec![DetectedLanguage {
                    language: self.default_language.clone(),
                    confidence: 0.2,
                    source: LanguageDetectionSource::ConfigDefault,
                }],
                hints: hint_vec,
                is_mixed: false,
                confidence: Some(0.2),
            };
        }

        let has_cyr = contains_cyrillic(trimmed);
        let has_lat = contains_latin(trimmed);

        if has_cyr && has_lat {
            let ru = cyrillic_ratio(trimmed).max(0.35);
            let en = latin_ratio(trimmed).max(0.35);
            return LanguageInfo {
                primary: Some(if ru >= en { "ru" } else { "en" }.to_string()),
                detected: vec![
                    DetectedLanguage {
                        language: "ru".to_string(),
                        confidence: ru,
                        source: LanguageDetectionSource::TextDetection,
                    },
                    DetectedLanguage {
                        language: "en".to_string(),
                        confidence: en,
                        source: LanguageDetectionSource::TextDetection,
                    },
                ],
                hints: hint_vec,
                is_mixed: true,
                confidence: Some(ru.max(en)),
            };
        }

        if has_cyr {
            return LanguageInfo {
                primary: Some("ru".to_string()),
                detected: vec![DetectedLanguage {
                    language: "ru".to_string(),
                    confidence: cyrillic_ratio(trimmed).max(0.8),
                    source: LanguageDetectionSource::TextDetection,
                }],
                hints: hint_vec,
                is_mixed: false,
                confidence: Some(0.9),
            };
        }

        if has_lat {
            return LanguageInfo {
                primary: Some("en".to_string()),
                detected: vec![DetectedLanguage {
                    language: "en".to_string(),
                    confidence: latin_ratio(trimmed).max(0.8),
                    source: LanguageDetectionSource::TextDetection,
                }],
                hints: hint_vec,
                is_mixed: false,
                confidence: Some(0.9),
            };
        }

        LanguageInfo {
            primary: Some(self.default_language.clone()),
            detected: vec![DetectedLanguage {
                language: self.default_language.clone(),
                confidence: 0.2,
                source: LanguageDetectionSource::ConfigDefault,
            }],
            hints: hint_vec,
            is_mixed: false,
            confidence: Some(0.2),
        }
    }
}

pub fn contains_cyrillic(text: &str) -> bool {
    text.chars().any(|ch| matches!(ch as u32, 0x0400..=0x04FF | 0x0500..=0x052F))
}

pub fn contains_latin(text: &str) -> bool {
    text.chars().any(|ch| ch.is_ascii_alphabetic())
}

pub fn cyrillic_ratio(text: &str) -> f32 {
    let mut letters = 0_usize;
    let mut cyr = 0_usize;
    for ch in text.chars() {
        if ch.is_alphabetic() {
            letters += 1;
            if matches!(ch as u32, 0x0400..=0x04FF | 0x0500..=0x052F) {
                cyr += 1;
            }
        }
    }
    if letters == 0 {
        0.0
    } else {
        cyr as f32 / letters as f32
    }
}

pub fn latin_ratio(text: &str) -> f32 {
    let mut letters = 0_usize;
    let mut lat = 0_usize;
    for ch in text.chars() {
        if ch.is_alphabetic() {
            letters += 1;
            if ch.is_ascii_alphabetic() {
                lat += 1;
            }
        }
    }
    if letters == 0 {
        0.0
    } else {
        lat as f32 / letters as f32
    }
}
