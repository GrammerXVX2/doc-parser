use document_parser::utils::russian_text::{
    normalize_dashes_ru, normalize_quotes_ru, normalize_russian_text, normalize_whitespace_ru,
};

#[test]
fn nbsp_to_space_and_collapse() {
    let input = "Привет\u{00A0}\u{00A0}мир";
    let out = normalize_whitespace_ru(input);
    assert_eq!(out, "Привет мир");
}

#[test]
fn yo_preserved_by_default() {
    let out = normalize_russian_text("ёлка");
    assert!(out.contains('ё'));
}

#[test]
fn quotes_and_dashes_conservative() {
    assert_eq!(normalize_quotes_ru("«тест»"), "\"тест\"");
    assert_eq!(normalize_dashes_ru("тест — ок"), "тест - ок");
}

#[test]
fn latin_terms_preserved() {
    let out = normalize_russian_text("Поддержка API и HTTP");
    assert!(out.contains("API"));
    assert!(out.contains("HTTP"));
}
