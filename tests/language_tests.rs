use document_parser::language::{HeuristicLanguageDetector, LanguageDetector};

#[test]
fn ru_detection() {
    let detector = HeuristicLanguageDetector::default();
    let info = detector.detect("Привет мир! Как дела?", &["ru".to_string(), "en".to_string()]);
    assert_eq!(info.primary.as_deref(), Some("ru"));
}

#[test]
fn en_detection() {
    let detector = HeuristicLanguageDetector {
        detect_min_chars: 1,
        ..Default::default()
    };
    let info = detector.detect("Hello world", &["ru".to_string(), "en".to_string()]);
    assert_eq!(info.primary.as_deref(), Some("en"));
}

#[test]
fn mixed_detection() {
    let detector = HeuristicLanguageDetector {
        detect_min_chars: 1,
        ..Default::default()
    };
    let info = detector.detect("Привет API world", &["ru".to_string(), "en".to_string()]);
    assert!(info.is_mixed);
}

#[test]
fn empty_text_fallback_ru() {
    let detector = HeuristicLanguageDetector::default();
    let info = detector.detect("", &["ru".to_string(), "en".to_string()]);
    assert_eq!(info.primary.as_deref(), Some("ru"));
}

#[test]
fn numbers_text_fallback_ru() {
    let detector = HeuristicLanguageDetector {
        detect_min_chars: 1,
        ..Default::default()
    };
    let info = detector.detect("12345", &["ru".to_string(), "en".to_string()]);
    assert_eq!(info.primary.as_deref(), Some("ru"));
}
