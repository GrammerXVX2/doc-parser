use crate::utils::russian_text::normalize_russian_text;

pub fn normalize_office_text(input: &str) -> String {
    normalize_russian_text(input)
}

pub fn is_heading_style(style: &str) -> bool {
    let s = style.to_lowercase();
    s.contains("heading") || s.contains("заголовок")
}

pub fn heading_level(style: &str) -> usize {
    let digits = style
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>();
    let level = digits.parse::<usize>().unwrap_or(1);
    level.clamp(1, 3)
}
