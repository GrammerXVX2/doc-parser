use serde::{Deserialize, Serialize};

use crate::model::{DocumentModel, ElementType};
use crate::runtime::pipeline_cli_overrides;

use super::config::ModelStackConfig;
use super::router::ModelRoutingDecision;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowPathDecision {
    pub should_run: bool,
    pub backend: Option<String>,
    pub alternatives: Vec<String>,
    pub reasons: Vec<String>,
    pub executed: bool,
}

pub fn decide_slow_path(
    model: &DocumentModel,
    routing: &ModelRoutingDecision,
    config: &ModelStackConfig,
) -> SlowPathDecision {
    let mut reasons = Vec::new();

    if !config.model_stack.routing.allow_slow_path {
        reasons.push("SLOW_PATH_SKIPPED_BY_CONFIG: slow path отключен в routing config".to_string());
        return SlowPathDecision {
            should_run: false,
            backend: routing.slow_path_backend.clone(),
            alternatives: slow_alternatives(routing),
            reasons,
            executed: false,
        };
    }

    if let Some(overrides) = pipeline_cli_overrides() {
        if let Some(false) = overrides.enable_slow_path {
            reasons.push("SLOW_PATH_SKIPPED_BY_CONFIG: отключено через CLI override".to_string());
            return SlowPathDecision {
                should_run: false,
                backend: routing.slow_path_backend.clone(),
                alternatives: slow_alternatives(routing),
                reasons,
                executed: false,
            };
        }
    }

    let threshold = config.model_stack.routing.slow_path_confidence_threshold;

    if min_observed_confidence(model) < threshold {
        reasons.push("SLOW_PATH_TRIGGERED: низкая уверенность OCR/layout".to_string());
    }

    if has_placeholder_tables(model) {
        reasons.push("SLOW_PATH_TRIGGERED: обнаружены placeholder tables".to_string());
    }

    if has_placeholder_formulas(model) {
        reasons.push("SLOW_PATH_TRIGGERED: обнаружены placeholder formulas".to_string());
    }

    if routing.selected_profile == "legal_high_accuracy" {
        reasons.push("SLOW_PATH_TRIGGERED: профиль legal_high_accuracy".to_string());
    }

    if routing.selected_profile.starts_with("legal") && legal_fields_missing(model) {
        reasons.push("SLOW_PATH_TRIGGERED: отсутствуют ключевые legal поля".to_string());
    }

    if let Some(overrides) = pipeline_cli_overrides() {
        if let Some(true) = overrides.enable_slow_path {
            reasons.push("SLOW_PATH_TRIGGERED: принудительно включено через CLI".to_string());
        }
    }

    let should_run = !reasons.is_empty();
    let executed = pipeline_cli_overrides()
        .and_then(|o| o.execute_slow_path)
        .unwrap_or(false)
        && should_run;

    SlowPathDecision {
        should_run,
        backend: routing.slow_path_backend.clone(),
        alternatives: slow_alternatives(routing),
        reasons,
        executed,
    }
}

fn slow_alternatives(routing: &ModelRoutingDecision) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(v) = &routing.vlm_backend {
        if routing.slow_path_backend.as_ref() != Some(v) {
            out.push(v.clone());
        }
    }
    out
}

fn min_observed_confidence(model: &DocumentModel) -> f32 {
    let mut min_conf = 1.0_f32;
    let mut found = false;

    for page in &model.pages {
        if let Some(v) = page
            .elements
            .iter()
            .filter_map(|el| el.confidence.get("overall"))
            .filter_map(|v| v.as_f64())
            .map(|v| v as f32)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        {
            min_conf = min_conf.min(v);
            found = true;
        }
    }

    if found { min_conf } else { 1.0 }
}

fn has_placeholder_tables(model: &DocumentModel) -> bool {
    model.pages.iter().any(|p| {
        p.elements.iter().any(|e| {
            e.element_type == ElementType::Table
                && (e.extra.get("detector_source").is_some()
                    || e.extra.get("detected_region_id").is_some()
                    || e.content
                        .get("text")
                        .and_then(|v| v.as_str())
                        .map(|v| v.contains("placeholder"))
                        .unwrap_or(false))
        })
    })
}

fn has_placeholder_formulas(model: &DocumentModel) -> bool {
    model.pages.iter().any(|p| {
        p.elements.iter().any(|e| {
            e.element_type == ElementType::Formula
                && (e.extra.get("detected_region_id").is_some()
                    || e.extra
                        .get("format")
                        .and_then(|v| v.as_str())
                        .map(|v| v == "unknown")
                        .unwrap_or(false))
        })
    })
}

fn legal_fields_missing(model: &DocumentModel) -> bool {
    let Some(legal) = model.extra.get("legal") else {
        return true;
    };

    let parties = legal
        .get("parties")
        .and_then(|v| v.as_array())
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    let dates = legal
        .get("dates")
        .and_then(|v| v.as_array())
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    let identifiers = legal
        .get("identifiers")
        .and_then(|v| v.as_array())
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    !(parties && dates && identifiers)
}
