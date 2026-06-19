use crate::utils::xml::extract_xml_texts;

pub fn extract_comments_texts(xml: &str) -> Vec<String> {
    extract_xml_texts(xml, "w:t")
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .collect()
}
