pub fn contains_omml(xml_fragment: &str) -> bool {
    xml_fragment.contains("<m:oMath") || xml_fragment.contains("<m:oMathPara")
}

pub fn extract_first_omml(xml_fragment: &str) -> Option<String> {
    if let Some(start) = xml_fragment.find("<m:oMath") {
        if let Some(end_rel) = xml_fragment[start..].find("</m:oMath>") {
            let end = start + end_rel + "</m:oMath>".len();
            return Some(xml_fragment[start..end].to_string());
        }
    }

    if let Some(start) = xml_fragment.find("<m:oMathPara") {
        if let Some(end_rel) = xml_fragment[start..].find("</m:oMathPara>") {
            let end = start + end_rel + "</m:oMathPara>".len();
            return Some(xml_fragment[start..end].to_string());
        }
    }

    None
}
