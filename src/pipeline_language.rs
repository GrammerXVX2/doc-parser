use serde_json::json;

use crate::config::PipelineConfig;
use crate::language::{HeuristicLanguageDetector, LanguageDetectionSource, LanguageDetector, LanguageInfo};
use crate::model::{DocumentModel, MessageLocale};
use crate::runtime::pipeline_cli_overrides;

pub fn apply_language_and_locale(model: &mut DocumentModel, config: &PipelineConfig) {
    let default_language = config
        .pipeline
        .language
        .get("default_language")
        .and_then(|v| v.as_str())
        .unwrap_or("ru")
        .to_string();

    let fallback_languages = config
        .pipeline
        .language
        .get("fallback_languages")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec!["ru".to_string(), "en".to_string()]);

    let detect_language = config
        .pipeline
        .language
        .get("detect_language")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let detect_min_chars = config
        .pipeline
        .language
        .get("detect_language_min_chars")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(40);

    let locale = resolve_locale(config);

    let mut effective_default = default_language.clone();
    let mut effective_fallback = fallback_languages.clone();
    let mut explicit_language_hint: Option<String> = None;
    if let Some(overrides) = pipeline_cli_overrides() {
        if let Some(lang) = &overrides.language {
            effective_default = lang.clone();
            explicit_language_hint = Some(lang.clone());
        }
        if let Some(langs) = &overrides.languages {
            if !langs.is_empty() {
                effective_fallback = langs.clone();
            }
        }
    }

    let detector = HeuristicLanguageDetector {
        default_language: effective_default.clone(),
        detect_min_chars,
    };

    let text = collect_document_text(model);
    let mut info = if detect_language {
        detector.detect(&text, &effective_fallback)
    } else {
        LanguageInfo {
            primary: Some(effective_default.clone()),
            detected: vec![crate::language::DetectedLanguage {
                language: effective_default.clone(),
                confidence: 1.0,
                source: LanguageDetectionSource::ConfigDefault,
            }],
            hints: effective_fallback.clone(),
            is_mixed: false,
            confidence: Some(1.0),
        }
    };
    if let Some(lang) = explicit_language_hint.as_deref() {
        info = force_primary_language(info, lang, &effective_fallback);
    }

    model.document_profile.languages = vec![info.primary.clone().unwrap_or(effective_default.clone())];
    model.document_profile.language_info = info.clone();

    for page in &mut model.pages {
        let mut page_info = if detect_language {
            detector.detect(&page.text, &effective_fallback)
        } else {
            info.clone()
        };
        if let Some(lang) = explicit_language_hint.as_deref() {
            page_info = force_primary_language(page_info, lang, &effective_fallback);
        }
        page.page_profile.language = page_info.primary.clone();
        page.page_profile.language_info = page_info.clone();

        for element in &mut page.elements {
            let element_text = element
                .content
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let mut element_info = if detect_language {
                detector.detect(element_text, &effective_fallback)
            } else {
                info.clone()
            };
            if let Some(lang) = explicit_language_hint.as_deref() {
                element_info = force_primary_language(element_info, lang, &effective_fallback);
            }
            element.extra.insert(
                "language_info".to_string(),
                serde_json::to_value(element_info).unwrap_or_else(|_| json!({})),
            );
        }
    }

    for chunk in &mut model.chunks {
        if chunk.metadata.get("language").is_none() || chunk.metadata["language"].is_null() {
            if let Some(primary) = &info.primary {
                chunk.metadata["language"] = json!(primary);
            }
        }
    }

    model.extra.insert("locale".to_string(), json!(match locale {
        MessageLocale::Ru => "ru",
        MessageLocale::En => "en",
    }));
}

fn force_primary_language(mut info: LanguageInfo, language: &str, hints: &[String]) -> LanguageInfo {
    let lang = language.to_string();
    info.primary = Some(lang.clone());
    info.hints = if hints.is_empty() {
        vec![lang.clone(), "en".to_string()]
    } else {
        hints.to_vec()
    };

    let already_present = info.detected.iter().any(|d| d.language == lang);
    if !already_present {
        info.detected.insert(
            0,
            crate::language::DetectedLanguage {
                language: lang,
                confidence: 1.0,
                source: LanguageDetectionSource::UserHint,
            },
        );
    }

    info
}

fn collect_document_text(model: &DocumentModel) -> String {
    model
        .pages
        .iter()
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn resolve_locale(config: &PipelineConfig) -> MessageLocale {
    if let Some(overrides) = pipeline_cli_overrides() {
        if let Some(locale) = &overrides.locale {
            if locale.eq_ignore_ascii_case("en") {
                return MessageLocale::En;
            }
            return MessageLocale::Ru;
        }
    }

    match config
        .pipeline
        .locale
        .get("default")
        .and_then(|v| v.as_str())
        .unwrap_or("ru")
        .to_ascii_lowercase()
        .as_str()
    {
        "en" => MessageLocale::En,
        _ => MessageLocale::Ru,
    }
}

pub fn localized_message(locale: MessageLocale, code: &str, fallback: &str) -> String {
    match locale {
        MessageLocale::Ru => match code {
            "LOW_OCR_CONFIDENCE" => "Низкая уверенность OCR-распознавания.".to_string(),
            "OCR_BACKEND_FALLBACK_TO_MOCK" => "ONNX OCR недоступен; используется mock OCR.".to_string(),
            "TABLE_PLACEHOLDER_CREATED" => {
                "На странице обнаружена таблица, но структура таблицы пока не распознана."
                    .to_string()
            }
            "FORMULA_PLACEHOLDER_CREATED" => {
                "Формула обнаружена, но распознавание пока не выполнено.".to_string()
            }
            "LAYOUT_DETECTION_FAILED" => {
                "Не удалось выполнить layout detection для страницы.".to_string()
            }
            "SCANNED_TABLE_DETECTION_FAILED" => {
                "Не удалось обнаружить таблицы на скане.".to_string()
            }
            "FORMULA_DETECTION_FAILED" => {
                "Не удалось обнаружить формулы на странице.".to_string()
            }
            "FORMULA_RECOGNITION_FAILED" => {
                "Не удалось распознать формулу для обнаруженного региона.".to_string()
            }
            "DEBUG_ARTIFACT_WRITE_FAILED" => {
                "Не удалось сохранить debug-артефакт layout/reading order.".to_string()
            }
            _ => fallback.to_string(),
        },
        MessageLocale::En => fallback.to_string(),
    }
}
