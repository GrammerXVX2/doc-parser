use std::path::PathBuf;

use document_parser::classifier::classify_file;
use document_parser::extractors::rtf::RtfExtractor;
use document_parser::router::Extractor;

#[test]
fn rtf_extractor_uses_fallback_chain_and_preserves_source() {
    let path = PathBuf::from("testdata/legacy/sample_ru.rtf");
    let classification = classify_file(&path).expect("classification");

    let model = RtfExtractor.extract(&path, &classification).expect("extract");

    assert_eq!(model.source.uri, path.to_string_lossy());

    if model.processing.status == document_parser::model::ProcessingStatus::Failed {
        assert!(model.errors.iter().any(|e| e.code == "RTF_CONVERSION_FAILED" || e.code == "RTF_NO_FALLBACK_AVAILABLE"));
        return;
    }

    let has_converted = model
        .pages
        .iter()
        .flat_map(|p| p.elements.iter())
        .any(|e| {
            e.provenance
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                == "converted"
        });
    assert!(has_converted);
}
