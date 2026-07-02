use regex::Regex;

use super::schema::BookChapter;

pub fn detect_chapters(text: &str) -> Vec<BookChapter> {
    let patterns = [
        Regex::new(r"(?im)^\s*глава\s+[0-9ivxlcdm]+[\.:]?\s*(.*)$").expect("valid regex"),
        Regex::new(r"(?im)^\s*часть\s+первая\s*(.*)$").expect("valid regex"),
        Regex::new(r"(?im)^\s*[ivxlcdm]+\.\s+(.+)$").expect("valid regex"),
        Regex::new(r"(?im)^\s*[0-9]+\.\s+(.+)$").expect("valid regex"),
    ];

    let mut out = Vec::new();
    for pattern in &patterns {
        for cap in pattern.captures_iter(text) {
            let title = cap
                .get(0)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            if title.is_empty() {
                continue;
            }
            out.push(BookChapter {
                title,
                index: Some(out.len() + 1),
            });
        }
    }

    dedup_chapters(out)
}

fn dedup_chapters(chapters: Vec<BookChapter>) -> Vec<BookChapter> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for mut chapter in chapters {
        if seen.insert(chapter.title.clone()) {
            chapter.index = Some(out.len() + 1);
            out.push(chapter);
        }
    }
    out
}
