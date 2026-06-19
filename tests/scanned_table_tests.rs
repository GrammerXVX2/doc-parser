use futures::executor::block_on;

use document_parser::converters::traits::ExtractionContext;
use document_parser::tables::{
    FixtureScannedTableDetector, ScannedTableDetector, TableDetectionInput,
    create_scanned_table_placeholder,
};

#[test]
fn fixture_table_region_creates_placeholder() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input = TableDetectionInput {
        document_id: "doc".to_string(),
        page_number: 1,
        page_image_path: Some(root.join("testdata/images/table_scan.png")),
        page_width: 1000.0,
        page_height: 1400.0,
    };

    let mut ctx = ExtractionContext::default();
    let detector = FixtureScannedTableDetector;
    let regions = block_on(detector.detect_tables(input, &mut ctx)).expect("fixture works");
    assert!(!regions.is_empty());

    let placeholder = create_scanned_table_placeholder(
        regions[0].page_number,
        &regions[0].region_id,
        regions[0].bbox,
        regions[0].confidence,
        &regions[0].source,
    );
    let text = placeholder
        .content
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(text.contains("Таблица обнаружена"));
    assert!(placeholder.confidence.get("overall").is_some());
}
