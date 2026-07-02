use super::rules::{
    extract_amounts, extract_citations, extract_dates, extract_identifiers, extract_parties,
};
use super::schema::{LegalClause, LegalExtraction, LegalRisk};
use crate::model::DocumentModel;

pub fn extract_legal_mvp(model: &DocumentModel) -> LegalExtraction {
    let text = model
        .pages
        .iter()
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let text_lower = text.to_lowercase();

    let document_type = if text_lower.contains("договор") || text_lower.contains("контракт") {
        Some("contract".to_string())
    } else if text_lower.contains("решение") || text_lower.contains("суд") {
        Some("court_act".to_string())
    } else {
        None
    };

    let mut clauses = Vec::new();
    if text_lower.contains("предмет") {
        clauses.push(LegalClause {
            title: Some("Предмет".to_string()),
            text: "Найден раздел с предметом договора".to_string(),
        });
    }

    let mut risks = Vec::new();
    if text_lower.contains("штраф") || text_lower.contains("неустойка") {
        risks.push(LegalRisk {
            name: "penalty_clause".to_string(),
            severity: Some("medium".to_string()),
        });
    }

    let parties = extract_parties(&text);
    let dates = extract_dates(&text);
    let identifiers = extract_identifiers(&text);
    let amounts = extract_amounts(&text);
    let citations = extract_citations(&text);

    let signal_count = parties.len() + dates.len() + identifiers.len() + amounts.len();
    let confidence = if signal_count == 0 {
        Some(0.25)
    } else {
        Some((0.45 + (signal_count as f32 * 0.07)).min(0.95))
    };

    LegalExtraction {
        document_type,
        parties,
        dates,
        amounts,
        identifiers,
        clauses,
        risks,
        citations,
        confidence,
    }
}

pub fn legal_required_fields_present(extraction: &LegalExtraction) -> bool {
    !extraction.parties.is_empty()
        && !extraction.dates.is_empty()
        && !extraction.identifiers.is_empty()
}
