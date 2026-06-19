use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct DocxStyles {
    pub heading_styles: HashSet<String>,
    pub monospace_styles: HashSet<String>,
}

pub fn parse_docx_styles(xml: &str) -> DocxStyles {
    let mut styles = DocxStyles::default();

    for chunk in xml.split("<w:style") {
        if !chunk.contains("</w:style>") {
            continue;
        }

        let style_id = attr_value(chunk, "w:styleId=")
            .or_else(|| attr_value(chunk, "styleId="))
            .unwrap_or_default();
        let style_name = scoped_attr_value(chunk, "<w:name", "w:val=")
            .or_else(|| scoped_attr_value(chunk, "<w:name", "val="))
            .unwrap_or_default();

        let marker = format!("{} {}", style_id, style_name).to_lowercase();
        if marker.contains("heading") || marker.contains("заголовок") {
            if !style_id.is_empty() {
                styles.heading_styles.insert(style_id.clone());
            }
            if !style_name.is_empty() {
                styles.heading_styles.insert(style_name);
            }
        }

        if chunk.contains("Courier") || chunk.contains("Consolas") || chunk.contains("Monospace") {
            if !style_id.is_empty() {
                styles.monospace_styles.insert(style_id);
            }
        }
    }

    styles
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
