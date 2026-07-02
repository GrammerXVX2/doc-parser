use document_parser::converters::traits::ExtractionContext;
use document_parser::models::backends::{
    ExtendedFormulaBackend, ExtendedFormulaInput, ExtendedOcrBackend, ExtendedOcrInput,
    ExtendedTableBackend, ExtendedTableInput, MockPaddleOcrV6Backend, MockPix2TexBackend,
    MockTableTransformerBackend, ModelBackend,
};

#[tokio::test]
async fn mock_ocr_backend_is_deterministic() {
    let backend = MockPaddleOcrV6Backend;
    let mut ctx = ExtractionContext::default();

    let input = ExtendedOcrInput {
        document_id: "doc1".to_string(),
        page_number: 1,
        image_path: None,
        languages: vec!["ru".to_string(), "en".to_string()],
    };

    let out1 = backend.run_ocr(input.clone(), &mut ctx).await.unwrap();
    let out2 = backend.run_ocr(input, &mut ctx).await.unwrap();

    assert_eq!(out1.confidence, out2.confidence);
    assert_eq!(out1.elements.len(), out2.elements.len());
}

#[tokio::test]
async fn mock_table_and_formula_backends_return_elements() {
    let table_backend = MockTableTransformerBackend;
    let formula_backend = MockPix2TexBackend;
    let mut ctx = ExtractionContext::default();

    let table = table_backend
        .recognize_table(
            ExtendedTableInput {
                document_id: "doc1".to_string(),
                page_number: 1,
                region_hint: None,
            },
            &mut ctx,
        )
        .await
        .unwrap();
    let formula = formula_backend
        .recognize_formula(
            ExtendedFormulaInput {
                document_id: "doc1".to_string(),
                page_number: 1,
                region_hint: None,
            },
            &mut ctx,
        )
        .await
        .unwrap();

    assert_eq!(table.element_id, "p1_table_1");
    assert_eq!(formula.element_id, "p1_formula_1");
}

#[tokio::test]
async fn mock_backend_health_check_is_available() {
    let backend = MockPaddleOcrV6Backend;
    let health = backend.health_check().await;
    assert!(health.available);
    assert!(health.message.unwrap_or_default().contains("mock"));
}
