pub fn omml_to_latex(omml: &str) -> Option<String> {
    let text = crate::utils::xml::strip_xml_tags(omml);
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}
