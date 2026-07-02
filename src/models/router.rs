use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::classifier::FileClassification;
use crate::model::DocumentModel;

use super::config::{ModelBackendConfig, ModelProfileConfig, ModelStackConfig};
use super::domain::{DocumentDomain, DomainProfile, detect_document_domain};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRoutingDecision {
    pub selected_profile: String,
    pub domain_profile: DomainProfile,
    pub fast_ocr_backend: Option<String>,
    pub structured_backend: Option<String>,
    pub vlm_backend: Option<String>,
    pub layout_backend: Option<String>,
    pub table_backend: Option<String>,
    pub formula_backend: Option<String>,
    pub legal_ner_backend: Option<String>,
    pub embedding_backend: Option<String>,
    pub book_backends: Vec<String>,
    pub slow_path_backend: Option<String>,
    pub reasons: Vec<String>,
}

pub fn route_models(
    classification: &FileClassification,
    model: Option<&DocumentModel>,
    config: &ModelStackConfig,
    user_profile_override: Option<&str>,
    user_domain_override: Option<&str>,
) -> ModelRoutingDecision {
    let mut reasons = Vec::new();

    let mut domain_profile = if let Some(m) = model {
        detect_document_domain(m)
    } else {
        DomainProfile {
            domain: DocumentDomain::Unknown,
            confidence: 0.2,
            reasons: vec!["domain detection skipped: model is absent".to_string()],
        }
    };

    if let Some(domain_override) = user_domain_override.and_then(parse_domain_override) {
        domain_profile = DomainProfile {
            domain: domain_override,
            confidence: 1.0,
            reasons: vec!["domain override from user".to_string()],
        };
    }

    let mut selected_profile = match user_profile_override {
        Some(profile) if config.model_stack.profiles.contains_key(profile) => {
            reasons.push("model profile override from user".to_string());
            profile.to_string()
        }
        Some(profile) => {
            reasons.push(format!(
                "MODEL_PROFILE_NOT_FOUND: profile '{profile}' is missing, fallback to auto routing"
            ));
            choose_profile_by_domain_and_format(classification, &domain_profile.domain)
        }
        None => choose_profile_by_domain_and_format(classification, &domain_profile.domain),
    };

    if !config.model_stack.profiles.contains_key(&selected_profile) {
        reasons.push(format!(
            "MODEL_PROFILE_NOT_FOUND: '{}' is missing, fallback to default profile",
            selected_profile
        ));
        selected_profile = config.model_stack.routing.default_profile.clone();
    }

    let profile = config
        .model_stack
        .profiles
        .get(&selected_profile)
        .cloned()
        .unwrap_or_default();

    let fast_ocr_backend = choose_backend_with_fallback(
        primary_ocr_backend(&profile),
        profile_fallbacks(&profile.ocr),
        &config.model_stack.backends,
        &mut reasons,
        "OCR_BACKEND_FALLBACK_USED",
    );

    let structured_backend = choose_backend_with_fallback(
        primary_structured_backend(&profile),
        profile_fallbacks(&profile.structured_document_parse),
        &config.model_stack.backends,
        &mut reasons,
        "MODEL_BACKEND_FALLBACK_USED",
    );

    let layout_backend = choose_backend_with_fallback(
        value_string(profile.layout.get("primary")).or_else(|| value_string(Some(&profile.layout))),
        profile_fallbacks(&profile.layout),
        &config.model_stack.backends,
        &mut reasons,
        "LAYOUT_BACKEND_FALLBACK_USED",
    );

    let table_backend = choose_backend_with_fallback(
        value_string(profile.tables.get("scanned")).or_else(|| value_string(Some(&profile.tables))),
        profile_fallbacks(&profile.tables),
        &config.model_stack.backends,
        &mut reasons,
        "TABLE_BACKEND_FALLBACK_USED",
    );

    let formula_backend = choose_backend_with_fallback(
        value_string(profile.formulas.get("scanned")).or_else(|| value_string(Some(&profile.formulas))),
        profile_fallbacks(&profile.formulas),
        &config.model_stack.backends,
        &mut reasons,
        "FORMULA_BACKEND_FALLBACK_USED",
    );

    let legal_ner_backend = choose_backend_with_fallback(
        value_string(nested(&profile.legal, &["ner", "primary"])),
        value_array_strings(nested(&profile.legal, &["ner", "fallback"])),
        &config.model_stack.backends,
        &mut reasons,
        "MODEL_BACKEND_FALLBACK_USED",
    );

    let embedding_backend = choose_backend_with_fallback(
        value_string(nested(&profile.legal, &["embeddings", "primary"])),
        value_array_strings(nested(&profile.legal, &["embeddings", "fallback"])),
        &config.model_stack.backends,
        &mut reasons,
        "MODEL_BACKEND_FALLBACK_USED",
    );

    let slow_path_backend = choose_backend_with_fallback(
        value_string(profile.slow_path.get("backend")),
        value_array_strings(profile.slow_path.get("alternatives")),
        &config.model_stack.backends,
        &mut reasons,
        "MODEL_BACKEND_FALLBACK_USED",
    );

    let vlm_backend = slow_path_backend.clone();

    let book_backends = if profile.book.get("enabled").is_some() || matches!(domain_profile.domain, DocumentDomain::Fiction | DocumentDomain::HistoricalBook) {
        let mut list = vec!["chapter_detection".to_string(), "dehyphenation".to_string()];
        if matches!(domain_profile.domain, DocumentDomain::HistoricalBook) {
            list.push("historical_orthography".to_string());
        }
        list
    } else {
        Vec::new()
    };

    ModelRoutingDecision {
        selected_profile,
        domain_profile,
        fast_ocr_backend,
        structured_backend,
        vlm_backend,
        layout_backend,
        table_backend,
        formula_backend,
        legal_ner_backend,
        embedding_backend,
        book_backends,
        slow_path_backend,
        reasons,
    }
}

fn choose_profile_by_domain_and_format(
    classification: &FileClassification,
    domain: &DocumentDomain,
) -> String {
    match domain {
        DocumentDomain::Legal => "legal_fast".to_string(),
        DocumentDomain::Fiction => "fiction_modern".to_string(),
        DocumentDomain::HistoricalBook => "fiction_historical".to_string(),
        DocumentDomain::Scientific => "scientific".to_string(),
        DocumentDomain::MixedEnterprise => "mixed_enterprise".to_string(),
        DocumentDomain::Unknown => {
            let _ = classification;
            "mixed_enterprise".to_string()
        }
    }
}

fn choose_backend_with_fallback(
    primary: Option<String>,
    fallbacks: Vec<String>,
    backends: &std::collections::HashMap<String, ModelBackendConfig>,
    reasons: &mut Vec<String>,
    fallback_code: &str,
) -> Option<String> {
    if let Some(primary) = primary {
        if backend_available(backends, &primary) {
            return Some(primary);
        }
        reasons.push(format!("{fallback_code}: primary backend is unavailable"));
    }

    for candidate in fallbacks {
        if backend_available(backends, &candidate) {
            return Some(candidate);
        }
    }

    None
}

fn backend_available(
    backends: &std::collections::HashMap<String, ModelBackendConfig>,
    name: &str,
) -> bool {
    backends.get(name).map(|b| b.enabled).unwrap_or(false)
}

fn primary_ocr_backend(profile: &ModelProfileConfig) -> Option<String> {
    if let Some(s) = value_string(profile.ocr.get("primary")) {
        return Some(s);
    }
    if let Some(v) = profile.ocr.get("primary") {
        if let Some(backend) = value_string(v.get("backend")) {
            return Some(backend);
        }
    }
    value_string(Some(&profile.ocr))
}

fn primary_structured_backend(profile: &ModelProfileConfig) -> Option<String> {
    value_string(profile.structured_document_parse.get("primary"))
        .or_else(|| value_string(Some(&profile.structured_document_parse)))
}

fn profile_fallbacks(value: &Value) -> Vec<String> {
    value_array_strings(value.get("fallback"))
}

fn value_string(value: Option<&Value>) -> Option<String> {
    value.and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        _ => None,
    })
}

fn value_array_strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn nested<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

pub fn parse_domain_override(input: &str) -> Option<DocumentDomain> {
    match input.trim().to_ascii_lowercase().as_str() {
        "legal" => Some(DocumentDomain::Legal),
        "fiction" => Some(DocumentDomain::Fiction),
        "historical_book" => Some(DocumentDomain::HistoricalBook),
        "scientific" => Some(DocumentDomain::Scientific),
        "mixed_enterprise" => Some(DocumentDomain::MixedEnterprise),
        "unknown" => Some(DocumentDomain::Unknown),
        _ => None,
    }
}
