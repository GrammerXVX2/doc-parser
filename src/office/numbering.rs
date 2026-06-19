use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct NumberingInfo {
    pub num_to_abstract: HashMap<String, String>,
    pub abstract_is_ordered: HashMap<String, bool>,
}

pub fn parse_numbering(xml: &str) -> NumberingInfo {
    let mut info = NumberingInfo::default();

    for part in xml.split("<w:abstractNum") {
        if !part.contains("</w:abstractNum>") {
            continue;
        }
        let abstract_id = attr_value(part, "w:abstractNumId=")
            .or_else(|| attr_value(part, "abstractNumId="))
            .unwrap_or_default();
        if abstract_id.is_empty() {
            continue;
        }

        let ordered = part.contains("<w:numFmt")
            && (part.contains("w:val=\"decimal\"")
                || part.contains("w:val=\"upperRoman\"")
                || part.contains("w:val=\"lowerRoman\"")
                || part.contains("w:val=\"upperLetter\"")
                || part.contains("w:val=\"lowerLetter\""));

        info.abstract_is_ordered.insert(abstract_id, ordered);
    }

    for part in xml.split("<w:num") {
        if !part.contains("</w:num>") {
            continue;
        }
        let num_id = attr_value(part, "w:numId=")
            .or_else(|| attr_value(part, "numId="))
            .unwrap_or_default();
        let abstract_id = scoped_attr_value(part, "<w:abstractNumId", "w:val=")
            .or_else(|| scoped_attr_value(part, "<w:abstractNumId", "val="))
            .unwrap_or_default();
        if !num_id.is_empty() && !abstract_id.is_empty() {
            info.num_to_abstract.insert(num_id, abstract_id);
        }
    }

    info
}

pub fn is_ordered_list(num_id: &str, info: &NumberingInfo) -> bool {
    let Some(abstract_id) = info.num_to_abstract.get(num_id) else {
        return false;
    };
    info.abstract_is_ordered
        .get(abstract_id)
        .copied()
        .unwrap_or(false)
}

fn attr_value(input: &str, marker: &str) -> Option<String> {
    let idx = input.find(marker)?;
    let rest = &input[idx + marker.len()..];
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let end = rest[1..].find(quote)?;
    Some(rest[1..1 + end].to_string())
}

fn scoped_attr_value(scope: &str, tag: &str, marker: &str) -> Option<String> {
    let tag_idx = scope.find(tag)?;
    let scoped = &scope[tag_idx..];
    attr_value(scoped, marker)
}
