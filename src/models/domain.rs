use serde::{Deserialize, Serialize};

use crate::model::DocumentModel;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentDomain {
    Legal,
    Fiction,
    HistoricalBook,
    Scientific,
    MixedEnterprise,
    Unknown,
}

impl DocumentDomain {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentDomain::Legal => "legal",
            DocumentDomain::Fiction => "fiction",
            DocumentDomain::HistoricalBook => "historical_book",
            DocumentDomain::Scientific => "scientific",
            DocumentDomain::MixedEnterprise => "mixed_enterprise",
            DocumentDomain::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainProfile {
    pub domain: DocumentDomain,
    pub confidence: f32,
    pub reasons: Vec<String>,
}

pub fn detect_document_domain(model: &DocumentModel) -> DomainProfile {
    let text = model
        .pages
        .iter()
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();

    let legal_markers = [
        "договор",
        "соглашение",
        "стороны",
        "заказчик",
        "исполнитель",
        "арендатор",
        "арендодатель",
        "истец",
        "ответчик",
        "суд",
        "решение",
        "постановление",
        "инн",
        "кпп",
        "огрн",
        "неустойка",
        "штраф",
        "обязательство",
        "реквизиты",
    ];

    let fiction_markers = [
        "глава",
        "роман",
        "рассказ",
        "стихотворение",
        "эпиграф",
        "диалог",
        "часть первая",
    ];

    let historical_markers = ["ѣ", "і", "ѳ", "ѵ", "дореформенная орфография", "старый скан"];

    let scientific_markers = [
        "формула",
        "теорема",
        "лемма",
        "уравнение",
        "доказательство",
        "рисунок",
        "таблица",
        "литература",
        "doi",
    ];

    let legal_hits = count_hits(&text, &legal_markers);
    let fiction_hits = count_hits(&text, &fiction_markers);
    let historical_hits = count_hits(&text, &historical_markers);
    let scientific_hits = count_hits(&text, &scientific_markers);

    let mixed_complex = model.stats.table_count > 0
        && model.stats.image_count > 0
        && model.stats.formula_count > 0
        && model.stats.text_element_count > 0;

    let mut reasons = Vec::new();

    if historical_hits > 0 {
        reasons.push("обнаружены признаки дореформенной орфографии".to_string());
        return DomainProfile {
            domain: DocumentDomain::HistoricalBook,
            confidence: confidence_from_hits(historical_hits, 0.7),
            reasons,
        };
    }

    if legal_hits >= fiction_hits && legal_hits >= scientific_hits && legal_hits > 0 {
        reasons.push(format!("обнаружены юридические маркеры: {legal_hits}"));
        return DomainProfile {
            domain: DocumentDomain::Legal,
            confidence: confidence_from_hits(legal_hits, 0.65),
            reasons,
        };
    }

    if scientific_hits >= fiction_hits && scientific_hits > 0 {
        reasons.push(format!("обнаружены научные маркеры: {scientific_hits}"));
        return DomainProfile {
            domain: DocumentDomain::Scientific,
            confidence: confidence_from_hits(scientific_hits, 0.62),
            reasons,
        };
    }

    if fiction_hits > 0 {
        reasons.push(format!("обнаружены литературные маркеры: {fiction_hits}"));
        return DomainProfile {
            domain: DocumentDomain::Fiction,
            confidence: confidence_from_hits(fiction_hits, 0.6),
            reasons,
        };
    }

    if mixed_complex {
        reasons.push("обнаружен смешанный состав: tables + images + formulas + text".to_string());
        return DomainProfile {
            domain: DocumentDomain::MixedEnterprise,
            confidence: 0.72,
            reasons,
        };
    }

    reasons.push("маркеры домена не обнаружены".to_string());
    DomainProfile {
        domain: DocumentDomain::Unknown,
        confidence: 0.3,
        reasons,
    }
}

fn count_hits(text: &str, markers: &[&str]) -> usize {
    markers.iter().filter(|m| text.contains(**m)).count()
}

fn confidence_from_hits(hits: usize, base: f32) -> f32 {
    let boost = (hits as f32 * 0.08).min(0.28);
    (base + boost).min(0.95)
}
