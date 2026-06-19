use document_parser::office::{Relationship, parse_relationships};

#[test]
fn parse_relationships_with_external_target() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.org" TargetMode="External"/>
</Relationships>"#;

    let rels = parse_relationships(xml).expect("parse relationships");
    assert_eq!(rels.len(), 2);

    let image = rels.iter().find(|r| r.id == "rId1").expect("image rel");
    assert_eq!(image.target, "media/image1.png");
    assert!(image.rel_type.contains("image"));

    let link = rels.iter().find(|r| r.id == "rId2").expect("link rel");
    assert_eq!(link.target_mode.as_deref(), Some("External"));
    assert_eq!(link.target, "https://example.org");

    let _: Vec<Relationship> = rels;
}
