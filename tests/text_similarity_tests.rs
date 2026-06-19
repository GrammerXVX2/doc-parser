use document_parser::merge::text_similarity;

#[test]
fn exact_match_is_one() {
    assert_eq!(text_similarity("hello world", "hello world"), 1.0);
}

#[test]
fn case_insensitive_match_is_high() {
    assert!(text_similarity("Invoice Number", "invoice number") >= 0.95);
}

#[test]
fn punctuation_insensitive_match_is_high() {
    assert!(text_similarity("total: $100.00", "total 100 00") >= 0.8);
}

#[test]
fn unrelated_text_is_low() {
    assert!(text_similarity("apple banana", "server kernel") < 0.4);
}

#[test]
fn substring_match_is_medium_high() {
    assert!(text_similarity("invoice number 12345", "number 12345") >= 0.8);
}
