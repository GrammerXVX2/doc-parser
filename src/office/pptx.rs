use std::collections::HashMap;

use crate::office::ooxml::OoxmlPackage;
use crate::office::relationships::{Relationship, parse_relationships};
use crate::utils::xml::extract_xml_texts;

#[derive(Debug, Clone)]
pub struct PresentationRef {
    pub slide_path: String,
    pub rel_id: String,
}

#[derive(Debug, Clone)]
pub struct ShapeTextBlock {
    pub is_title: bool,
    pub text: String,
    pub list_items: Vec<String>,
    pub embed_rids: Vec<String>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

pub fn emu_to_px(emu: i64) -> f32 {
    // 1 px ~= 9525 EMU at 96 DPI.
    emu as f32 / 9525.0
}

pub fn parse_presentation_slide_refs(package: &OoxmlPackage) -> anyhow::Result<Vec<PresentationRef>> {
    let presentation_xml = package
        .read_text("ppt/presentation.xml")?
        .ok_or_else(|| anyhow::anyhow!("PPTX_PRESENTATION_XML_MISSING"))?;

    let rels_xml = package
        .read_text("ppt/_rels/presentation.xml.rels")?
        .ok_or_else(|| anyhow::anyhow!("PPTX_RELATIONSHIP_MISSING"))?;

    let rels = parse_relationships(&rels_xml)?;
    let rel_map = rels
        .iter()
        .map(|r| (r.id.clone(), r.target.clone()))
        .collect::<HashMap<_, _>>();

    let slide_refs = extract_slide_rel_ids(&presentation_xml)
        .into_iter()
        .filter_map(|rel_id| {
            rel_map.get(&rel_id).map(|target| PresentationRef {
                slide_path: normalize_target("ppt", target),
                rel_id,
            })
        })
        .collect::<Vec<_>>();

    Ok(slide_refs)
}

pub fn parse_slide_relationships(package: &OoxmlPackage, slide_path: &str) -> Vec<Relationship> {
    let rels_path = slide_rels_path(slide_path);
    match package.read_text(&rels_path) {
        Ok(Some(xml)) => parse_relationships(&xml).unwrap_or_default(),
        _ => Vec::new(),
    }
}

pub fn extract_notes_path(rels: &[Relationship]) -> Option<String> {
    rels.iter()
        .find(|r| r.rel_type.contains("notesSlide"))
        .map(|r| normalize_target("ppt/slides", &r.target))
}

pub fn extract_chart_rids(rels: &[Relationship]) -> Vec<String> {
    rels.iter()
        .filter(|r| r.rel_type.contains("chart"))
        .map(|r| r.id.clone())
        .collect()
}

pub fn resolve_embed_target(rels: &[Relationship], rid: &str) -> Option<String> {
    rels.iter()
        .find(|r| r.id == rid)
        .map(|r| normalize_target("ppt/slides", &r.target))
}

pub fn parse_slide_shape_blocks(slide_xml: &str) -> Vec<ShapeTextBlock> {
    let mut out = Vec::new();

    for (shape_idx, shape_xml) in extract_blocks(slide_xml, "p:sp").iter().enumerate() {
        let paragraphs = extract_blocks(shape_xml, "a:p");
        let mut text_lines = Vec::new();
        let mut list_items = Vec::new();

        for p_xml in &paragraphs {
            let line = extract_xml_texts(p_xml, "a:t").join("");
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }
            if is_list_paragraph(p_xml, &line) {
                list_items.push(line.clone());
            }
            text_lines.push(line);
        }

        let text = text_lines.join("\n");
        if text.is_empty() && list_items.is_empty() {
            continue;
        }

        let (x, y, w, h) = extract_shape_bbox(shape_xml);
        out.push(ShapeTextBlock {
            is_title: is_title_placeholder(shape_xml) || shape_idx == 0,
            text,
            list_items,
            embed_rids: extract_embed_rids(shape_xml),
            x,
            y,
            width: w,
            height: h,
        });
    }

    out
}

pub fn extract_slide_tables(slide_xml: &str) -> Vec<Vec<Vec<String>>> {
    let mut tables = Vec::new();

    for tbl_xml in extract_blocks(slide_xml, "a:tbl") {
        let rows = extract_blocks(&tbl_xml, "a:tr")
            .into_iter()
            .map(|tr_xml| {
                extract_blocks(&tr_xml, "a:tc")
                    .into_iter()
                    .map(|tc_xml| extract_xml_texts(&tc_xml, "a:t").join(""))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        if !rows.is_empty() {
            tables.push(rows);
        }
    }

    tables
}

pub fn extract_notes_text(notes_xml: &str) -> String {
    extract_xml_texts(notes_xml, "a:t")
        .into_iter()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn extract_slide_embed_rids(slide_xml: &str) -> Vec<String> {
    extract_embed_rids(slide_xml)
}

fn extract_slide_rel_ids(presentation_xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let marker = "r:id=";
    let mut cursor = presentation_xml;
    while let Some(idx) = cursor.find("<p:sldId") {
        cursor = &cursor[idx..];
        if let Some(id_idx) = cursor.find(marker) {
            let value = &cursor[id_idx + marker.len()..];
            if let Some(quote) = value.chars().next() {
                if quote == '"' || quote == '\'' {
                    if let Some(end) = value[1..].find(quote) {
                        out.push(value[1..1 + end].to_string());
                    }
                }
            }
        }
        cursor = &cursor[6..];
    }
    out
}

fn slide_rels_path(slide_path: &str) -> String {
    let mut parts = slide_path.split('/').collect::<Vec<_>>();
    let file_name = parts.pop().unwrap_or("slide1.xml");
    let base = parts.join("/");
    format!("{}/_rels/{}.rels", base, file_name)
}

fn normalize_target(base_dir: &str, target: &str) -> String {
    if target.starts_with('/') {
        return target.trim_start_matches('/').to_string();
    }

    let mut base = base_dir
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    for segment in target.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            if !base.is_empty() {
                base.pop();
            }
        } else {
            base.push(segment.to_string());
        }
    }

    base.join("/")
}

fn extract_blocks(xml: &str, tag: &str) -> Vec<String> {
    let mut out = Vec::new();
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let mut cursor = 0usize;

    while let Some(start_rel) = xml[cursor..].find(&open) {
        let start = cursor + start_rel;
        if let Some(open_end_rel) = xml[start..].find('>') {
            let open_end = start + open_end_rel;
            if xml[open_end.saturating_sub(1)..=open_end].starts_with("/>") {
                out.push(xml[start..=open_end].to_string());
                cursor = open_end + 1;
                continue;
            }
            if let Some(end_rel) = xml[open_end + 1..].find(&close) {
                let end = open_end + 1 + end_rel + close.len();
                out.push(xml[start..end].to_string());
                cursor = end;
                continue;
            }
        }
        break;
    }

    out
}

fn is_title_placeholder(shape_xml: &str) -> bool {
    shape_xml.contains("<p:ph")
        && (shape_xml.contains("type=\"title\"")
            || shape_xml.contains("type=\"ctrTitle\""))
}

fn is_list_paragraph(paragraph_xml: &str, line: &str) -> bool {
    paragraph_xml.contains("<a:bu")
        || line.starts_with('-')
        || line.starts_with('•')
        || line.starts_with('*')
}

fn extract_embed_rids(shape_xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let marker = "r:embed=";
    let mut cursor = shape_xml;

    while let Some(idx) = cursor.find(marker) {
        let value = &cursor[idx + marker.len()..];
        if let Some(quote) = value.chars().next() {
            if quote == '"' || quote == '\'' {
                if let Some(end) = value[1..].find(quote) {
                    out.push(value[1..1 + end].to_string());
                    cursor = &value[1 + end..];
                    continue;
                }
            }
        }
        break;
    }

    out
}

fn extract_shape_bbox(shape_xml: &str) -> (Option<f32>, Option<f32>, Option<f32>, Option<f32>) {
    let x = parse_emu_attr(shape_xml, "<a:off", "x=");
    let y = parse_emu_attr(shape_xml, "<a:off", "y=");
    let w = parse_emu_attr(shape_xml, "<a:ext", "cx=");
    let h = parse_emu_attr(shape_xml, "<a:ext", "cy=");
    (x, y, w, h)
}

fn parse_emu_attr(scope: &str, tag: &str, attr: &str) -> Option<f32> {
    let tag_idx = scope.find(tag)?;
    let scoped = &scope[tag_idx..];
    let attr_idx = scoped.find(attr)?;
    let value = &scoped[attr_idx + attr.len()..];
    let quote = value.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let end = value[1..].find(quote)?;
    let emu = value[1..1 + end].parse::<i64>().ok()?;
    Some(emu_to_px(emu))
}
