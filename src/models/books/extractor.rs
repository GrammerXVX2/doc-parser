use regex::Regex;

use crate::model::DocumentModel;

use super::chapter_detection::detect_chapters;
use super::dehyphenation::apply_dehyphenation;
use super::schema::{BookExtraction, BookFootnote, TocEntry};

pub fn extract_book_mvp(model: &DocumentModel) -> BookExtraction {
    let text = model
        .pages
        .iter()
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let (dehyphenated, dehyphenation_applied) = apply_dehyphenation(&text);
    let chapters = detect_chapters(&dehyphenated);
    let footnotes = detect_footnotes(&dehyphenated);
    let toc = build_toc(&chapters);

    let title = dehyphenated
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned);

    BookExtraction {
        title,
        authors: Vec::new(),
        chapters,
        footnotes,
        toc,
        dehyphenation_applied,
        historical_orthography_detected: detect_historical_orthography(&dehyphenated),
    }
}

fn detect_footnotes(text: &str) -> Vec<BookFootnote> {
    let re = Regex::new(r"(?m)^\s*(\[?[0-9]{1,2}\]?)[\)\.]\s+(.+)$").expect("valid regex");
    re.captures_iter(text)
        .filter_map(|cap| {
            let marker = cap.get(1)?.as_str().to_string();
            let body = cap.get(2)?.as_str().to_string();
            Some(BookFootnote { marker, text: body })
        })
        .collect()
}

fn build_toc(chapters: &[super::schema::BookChapter]) -> Vec<TocEntry> {
    chapters
        .iter()
        .enumerate()
        .map(|(i, chapter)| TocEntry {
            title: chapter.title.clone(),
            page: Some((i + 1) as u32),
        })
        .collect()
}

pub fn detect_historical_orthography(text: &str) -> bool {
    let lowered = text.to_lowercase();
    ["ѣ", "і", "ѳ", "ѵ"].iter().any(|m| lowered.contains(m))
}
