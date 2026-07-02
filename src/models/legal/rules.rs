use regex::Regex;

use super::schema::{
    LegalAmount, LegalCitation, LegalDate, LegalIdentifier, LegalParty,
};

pub fn extract_identifiers(text: &str) -> Vec<LegalIdentifier> {
    let mut out = Vec::new();

    push_regex_identifiers(text, "ИНН", r"(?i)инн\s*[:№]?\s*([0-9]{10,12})", &mut out);
    push_regex_identifiers(text, "КПП", r"(?i)кпп\s*[:№]?\s*([0-9]{9})", &mut out);
    push_regex_identifiers(text, "ОГРН", r"(?i)огрн\s*[:№]?\s*([0-9]{13,15})", &mut out);
    push_regex_identifiers(
        text,
        "contract_number",
        r"(?i)(договор|контракт)\s*(?:№|N|#)\s*([А-ЯA-Za-z0-9\-\/]+)",
        &mut out,
    );

    dedup_identifiers(out)
}

pub fn extract_dates(text: &str) -> Vec<LegalDate> {
    let re = Regex::new(r"\b([0-3]?\d\.[0-1]?\d\.(?:19|20)?\d\d)\b").expect("valid regex");
    re.captures_iter(text)
        .filter_map(|cap| cap.get(1).map(|m| LegalDate { value: m.as_str().to_string() }))
        .collect()
}

pub fn extract_amounts(text: &str) -> Vec<LegalAmount> {
    let re = Regex::new(
        r"\b([0-9]{1,3}(?:[\s\u{00A0}]?[0-9]{3})*(?:[\.,][0-9]{1,2})?)\s*(руб\.?|р\.|RUB|₽)?\b",
    )
    .expect("valid regex");

    re.captures_iter(text)
        .filter_map(|cap| {
            let value = cap.get(1)?.as_str().to_string();
            if value.chars().all(|c| c.is_ascii_digit()) && value.len() < 4 {
                return None;
            }
            let currency = cap.get(2).map(|m| m.as_str().to_string());
            Some(LegalAmount { value, currency })
        })
        .collect()
}

pub fn extract_parties(text: &str) -> Vec<LegalParty> {
    let mut out = Vec::new();

    for marker in [
        "Заказчик",
        "Исполнитель",
        "Арендатор",
        "Арендодатель",
        "Истец",
        "Ответчик",
    ] {
        if text.to_lowercase().contains(&marker.to_lowercase()) {
            out.push(LegalParty {
                role: Some(marker.to_string()),
                name: None,
            });
        }
    }

    out
}

pub fn extract_citations(text: &str) -> Vec<LegalCitation> {
    let re = Regex::new(r"(?i)(ст\.?\s*[0-9]+(?:\.[0-9]+)?\s*(?:гк\s*рф|ук\s*рф|коап\s*рф)?)")
        .expect("valid regex");
    re.captures_iter(text)
        .filter_map(|cap| cap.get(1).map(|m| LegalCitation { value: m.as_str().to_string() }))
        .collect()
}

fn push_regex_identifiers(text: &str, kind: &str, pattern: &str, out: &mut Vec<LegalIdentifier>) {
    let re = Regex::new(pattern).expect("valid regex");
    for cap in re.captures_iter(text) {
        let value = cap
            .get(2)
            .or_else(|| cap.get(1))
            .map(|m| m.as_str().to_string());
        if let Some(value) = value {
            out.push(LegalIdentifier {
                kind: kind.to_string(),
                value,
            });
        }
    }
}

fn dedup_identifiers(items: Vec<LegalIdentifier>) -> Vec<LegalIdentifier> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for item in items {
        let key = format!("{}:{}", item.kind, item.value);
        if seen.insert(key) {
            out.push(item);
        }
    }
    out
}
