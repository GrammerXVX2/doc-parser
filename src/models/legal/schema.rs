use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegalParty {
    pub role: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegalDate {
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegalAmount {
    pub value: String,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegalIdentifier {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegalClause {
    pub title: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegalRisk {
    pub name: String,
    pub severity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegalCitation {
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegalExtraction {
    pub document_type: Option<String>,
    pub parties: Vec<LegalParty>,
    pub dates: Vec<LegalDate>,
    pub amounts: Vec<LegalAmount>,
    pub identifiers: Vec<LegalIdentifier>,
    pub clauses: Vec<LegalClause>,
    pub risks: Vec<LegalRisk>,
    pub citations: Vec<LegalCitation>,
    pub confidence: Option<f32>,
}
