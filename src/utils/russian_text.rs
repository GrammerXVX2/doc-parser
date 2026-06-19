#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RussianNormalizationMode {
    Conservative,
    Search,
    Aggressive,
}

impl RussianNormalizationMode {
    pub fn from_str(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "aggressive" => Self::Aggressive,
            "search" => Self::Search,
            _ => Self::Conservative,
        }
    }
}

pub fn normalize_russian_text(input: &str) -> String {
    let text = normalize_whitespace_ru(input);
    let text = normalize_quotes_ru(&text);
    let text = normalize_dashes_ru(&text);
    normalize_ocr_common_errors_ru(&text)
}

pub fn normalize_ocr_common_errors_ru(input: &str) -> String {
    // Conservative policy: keep letters and symbols as-is, only fix safe punctuation spacing.
    input
        .replace(" ,", ",")
        .replace(" .", ".")
        .replace(" :", ":")
        .replace(" ;", ";")
}

pub fn normalize_quotes_ru(input: &str) -> String {
    input
        .replace('“', "\"")
        .replace('”', "\"")
        .replace('„', "\"")
        .replace('«', "\"")
        .replace('»', "\"")
        .replace('’', "'")
        .replace('‘', "'")
}

pub fn normalize_dashes_ru(input: &str) -> String {
    input
        .replace('—', "-")
        .replace('–', "-")
        .replace('−', "-")
}

pub fn normalize_whitespace_ru(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_space = false;

    for ch in input.chars() {
        let mapped = match ch {
            '\u{00A0}' | '\u{2007}' | '\u{202F}' => ' ',
            _ => ch,
        };

        if mapped.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(mapped);
            prev_space = false;
        }
    }

    out.trim().to_string()
}
