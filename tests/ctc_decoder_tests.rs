use document_parser::ocr::decoder::ctc_greedy_decode_indices;

#[test]
fn ctc_removes_blanks_and_duplicates() {
    let charset = vec!["_".to_string(), "a".to_string(), "b".to_string()];
    let decoded = ctc_greedy_decode_indices(&[1, 1, 0, 2, 2], &charset, 0);
    assert_eq!(decoded.text, "ab");
}

#[test]
fn ctc_handles_unknown_index() {
    let charset = vec!["_".to_string(), "a".to_string()];
    let decoded = ctc_greedy_decode_indices(&[1, 3, 0, 1], &charset, 0);
    assert_eq!(decoded.text, "aa");
}
