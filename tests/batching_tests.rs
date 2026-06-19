use document_parser::ocr::dynamic_batcher::chunk_batches;

#[test]
fn chunk_batches_empty_input() {
    let batches: Vec<Vec<i32>> = chunk_batches(vec![], 64);
    assert!(batches.is_empty());
}

#[test]
fn chunk_batches_single_item() {
    let batches = chunk_batches(vec![1], 64);
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0], vec![1]);
}

#[test]
fn chunk_batches_respects_max_size() {
    let input: Vec<usize> = (0..65).collect();
    let batches = chunk_batches(input, 64);
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].len(), 64);
    assert_eq!(batches[1].len(), 1);
}
