use regex::Regex;

#[derive(Debug, Clone)]
pub enum DocxBlockKind {
    Paragraph,
    Table,
}

#[derive(Debug, Clone)]
pub struct DocxBlock {
    pub kind: DocxBlockKind,
    pub start: usize,
    pub xml: String,
}

pub fn extract_ordered_blocks(document_xml: &str) -> anyhow::Result<Vec<DocxBlock>> {
    let table_re = Regex::new(r"(?s)<w:tbl\b[^>]*>.*?</w:tbl>")?;
    let para_re = Regex::new(r"(?s)<w:p\b[^>]*>.*?</w:p>")?;

    let table_spans = table_re
        .find_iter(document_xml)
        .map(|m| (m.start(), m.end(), m.as_str().to_string()))
        .collect::<Vec<_>>();

    let mut blocks = table_spans
        .iter()
        .map(|(start, _end, xml)| DocxBlock {
            kind: DocxBlockKind::Table,
            start: *start,
            xml: xml.clone(),
        })
        .collect::<Vec<_>>();

    for m in para_re.find_iter(document_xml) {
        let inside_table = table_spans
            .iter()
            .any(|(s, e, _)| m.start() >= *s && m.end() <= *e);
        if inside_table {
            continue;
        }
        blocks.push(DocxBlock {
            kind: DocxBlockKind::Paragraph,
            start: m.start(),
            xml: m.as_str().to_string(),
        });
    }

    blocks.sort_by_key(|b| b.start);
    Ok(blocks)
}
