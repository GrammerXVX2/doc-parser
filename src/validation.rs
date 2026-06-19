use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::model::{DocumentModel, ElementType};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationSeverity {
    Warning,
    Error,
    Fatal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub code: String,
    pub severity: ValidationSeverity,
    pub message: String,
}

pub fn validate_document_model(document: &DocumentModel) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    if document.document_id.trim().is_empty() {
        issues.push(issue(
            "DOCUMENT_ID_EMPTY",
            ValidationSeverity::Fatal,
            "document_id is empty",
        ));
    }
    if document.schema_version.trim().is_empty() {
        issues.push(issue(
            "SCHEMA_VERSION_EMPTY",
            ValidationSeverity::Fatal,
            "schema_version is empty",
        ));
    }
    if document.stats.page_count != document.pages.len() as u32 {
        issues.push(issue(
            "PAGE_COUNT_MISMATCH",
            ValidationSeverity::Error,
            "stats.page_count does not match pages length",
        ));
    }

    if document.document_profile.languages.is_empty() {
        issues.push(issue(
            "DOCUMENT_LANGUAGE_EMPTY",
            ValidationSeverity::Warning,
            "document_profile.languages should include at least one language",
        ));
    }

    let mut element_ids = HashSet::new();
    let mut asset_ids = HashSet::new();
    let mut global_orders = HashSet::new();

    for asset in &document.assets {
        if !asset_ids.insert(asset.asset_id.clone()) {
            issues.push(issue(
                "DUPLICATE_ASSET_ID",
                ValidationSeverity::Error,
                "asset IDs must be unique",
            ));
        }
    }

    let asset_lookup = document
        .assets
        .iter()
        .map(|a| a.asset_id.clone())
        .collect::<HashSet<_>>();

    for page in &document.pages {
        if page.page_profile.language.is_none() {
            issues.push(issue(
                "PAGE_LANGUAGE_EMPTY",
                ValidationSeverity::Warning,
                "page_profile.language should be set",
            ));
        }
        let mut local_orders: Vec<u32> = Vec::new();
        for element in &page.elements {
            if !element_ids.insert(element.element_id.clone()) {
                issues.push(issue(
                    "DUPLICATE_ELEMENT_ID",
                    ValidationSeverity::Error,
                    "element IDs must be unique",
                ));
            }

            if element.provenance.is_null() {
                issues.push(issue(
                    "ELEMENT_MISSING_PROVENANCE",
                    ValidationSeverity::Error,
                    "each element must include provenance",
                ));
            }
            if element.confidence.is_null() {
                issues.push(issue(
                    "ELEMENT_MISSING_CONFIDENCE",
                    ValidationSeverity::Error,
                    "each element must include confidence",
                ));
            }

            if matches!(
                element.element_type,
                ElementType::Text
                    | ElementType::TextOcr
                    | ElementType::Heading
                    | ElementType::Paragraph
                    | ElementType::Blockquote
                    | ElementType::Code
                    | ElementType::List
                    | ElementType::ListItem
            ) {
                let has_text = element
                    .content
                    .get("text")
                    .and_then(|v| v.as_str())
                    .map(|v| !v.trim().is_empty())
                    .unwrap_or(false);
                let has_html = element
                    .content
                    .get("html")
                    .and_then(|v| v.as_str())
                    .map(|v| !v.trim().is_empty())
                    .unwrap_or(false);
                if !has_text && !has_html {
                    issues.push(issue(
                        "TEXT_ELEMENT_CONTENT_EMPTY",
                        ValidationSeverity::Error,
                        "text-like element must have content.text or content.html",
                    ));
                }
            }

            if let Some(local) = element.reading_order {
                local_orders.push(local);
            }
            if let Some(global) = element.global_order {
                if !global_orders.insert(global) {
                    issues.push(issue(
                        "GLOBAL_ORDER_DUPLICATE",
                        ValidationSeverity::Error,
                        "global_order values must be unique",
                    ));
                }
            }

            if let Some(asset_id) = element.extra.get("asset_id").and_then(|v| v.as_str()) {
                if !asset_lookup.contains(asset_id) {
                    issues.push(issue(
                        "ASSET_REFERENCE_NOT_FOUND",
                        ValidationSeverity::Error,
                        "element references missing asset_id",
                    ));
                }
            }
        }

        if !is_non_decreasing(&local_orders) {
            issues.push(issue(
                "READING_ORDER_INVALID",
                ValidationSeverity::Error,
                "reading_order must be non-decreasing within a page",
            ));
        }
    }

    let known_elements = element_ids;
    let mut missing_chunk_refs: HashMap<String, usize> = HashMap::new();
    for chunk in &document.chunks {
        for id in &chunk.element_ids {
            if !known_elements.contains(id) {
                *missing_chunk_refs.entry(id.clone()).or_insert(0) += 1;
            }
        }
    }

    if !missing_chunk_refs.is_empty() {
        issues.push(issue(
            "CHUNK_ELEMENT_REF_MISSING",
            ValidationSeverity::Error,
            "chunk references unknown element_ids",
        ));
    }

    issues
}

fn is_non_decreasing(values: &[u32]) -> bool {
    if values.is_empty() {
        return true;
    }
    let mut prev = values[0];
    for value in &values[1..] {
        if *value < prev {
            return false;
        }
        prev = *value;
    }
    true
}

fn issue(code: &str, severity: ValidationSeverity, message: &str) -> ValidationIssue {
    ValidationIssue {
        code: code.to_string(),
        severity,
        message: message.to_string(),
    }
}
