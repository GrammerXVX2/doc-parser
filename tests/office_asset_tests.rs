use std::path::PathBuf;

use document_parser::classifier::classify_file;
use document_parser::extractors::docx::DocxExtractor;
use document_parser::extractors::xlsx::XlsxExtractor;
use document_parser::router::Extractor;

#[test]
fn docx_embedded_image_assets_are_saved_and_referenced() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("testdata/office/sample_ru.docx");
    let classification = classify_file(&path).expect("classification");

    let model = DocxExtractor
        .extract(&path, &classification)
        .expect("docx extraction");

    let image_assets = model
        .assets
        .iter()
        .filter(|a| a.asset_type == "embedded_image")
        .collect::<Vec<_>>();
    assert!(!image_assets.is_empty());

    let refs = model
        .pages
        .iter()
        .flat_map(|p| p.elements.iter())
        .filter_map(|e| e.extra.get("asset_id").and_then(|v| v.as_str()))
        .collect::<Vec<_>>();
    assert!(refs.iter().any(|id| image_assets.iter().any(|a| a.asset_id == *id)));
}

#[test]
fn xlsx_embedded_image_assets_are_saved_when_present() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("testdata/office/sample_finance.xlsx");
    let classification = classify_file(&path).expect("classification");

    let model = XlsxExtractor
        .extract(&path, &classification)
        .expect("xlsx extraction");

    let image_assets = model
        .assets
        .iter()
        .filter(|a| a.asset_type == "embedded_image")
        .collect::<Vec<_>>();
    assert!(!image_assets.is_empty());
}
