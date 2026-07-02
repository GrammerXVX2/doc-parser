use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BookChapter {
    pub title: String,
    pub index: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BookFootnote {
    pub marker: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TocEntry {
    pub title: String,
    pub page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BookExtraction {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub chapters: Vec<BookChapter>,
    pub footnotes: Vec<BookFootnote>,
    pub toc: Vec<TocEntry>,
    pub dehyphenation_applied: bool,
    pub historical_orthography_detected: bool,
}
