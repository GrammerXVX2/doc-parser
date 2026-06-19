use futures::executor::block_on;

use document_parser::converters::traits::ExtractionContext;
use document_parser::layout::{
    FixtureLayoutDetector, HeuristicLayoutDetector, LayoutDetectionInput, LayoutDetector,
    MockLayoutDetector,
};
use document_parser::model::{Element, ElementType};

fn base_input() -> LayoutDetectionInput {
    LayoutDetectionInput {
        document_id: "doc".to_string(),
        page_number: 1,
        page_image_asset_id: None,
        page_image_path: None,
        page_width: 1000.0,
        page_height: 1400.0,
        existing_elements: vec![Element {
            element_id: "e1".to_string(),
            element_type: ElementType::Table,
            tag: Some("table".to_string()),
            role: None,
            reading_order: Some(1),
            global_order: None,
            bbox: Some([100.0, 200.0, 900.0, 600.0]),
            polygon: None,
            content: serde_json::json!({"text":"t"}),
            style: serde_json::json!({}),
            provenance: serde_json::json!({}),
            confidence: serde_json::json!({"overall":0.9}),
            warnings: vec![],
            extra: Default::default(),
        }],
    }
}

#[test]
fn mock_layout_returns_regions() {
    let detector = MockLayoutDetector;
    let mut ctx = ExtractionContext::default();
    let regions = block_on(detector.detect_layout(base_input(), &mut ctx)).expect("mock works");
    assert!(!regions.is_empty());
    assert!(regions.iter().all(|r| r.confidence > 0.0));
}

#[test]
fn heuristic_detects_table_region() {
    let detector = HeuristicLayoutDetector;
    let mut ctx = ExtractionContext::default();
    let regions = block_on(detector.detect_layout(base_input(), &mut ctx)).expect("heuristic works");
    assert!(regions.iter().any(|r| r.region_type.as_str() == "table"));
}

#[test]
fn fixture_layout_loads_json() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let detector = FixtureLayoutDetector;
    let mut ctx = ExtractionContext::default();
    let mut input = base_input();
    input.page_image_path = Some(root.join("testdata/images/formula_scan.png"));

    let regions = block_on(detector.detect_layout(input, &mut ctx)).expect("fixture works");
    assert!(regions.iter().any(|r| r.region_type.as_str() == "formula"));
}
