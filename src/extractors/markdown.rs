use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::Context;
use pulldown_cmark::{Options, Parser, html};
use serde_json::json;

use crate::classifier::FileClassification;
use crate::extractors::{
    base_document_model, default_confidence, empty_style, provenance, stage, update_stats,
};
use crate::model::{ContentMode, DocumentFormat, Element, ElementType, PageType};
use crate::router::Extractor;
use crate::tables::{TableStructure, detect_pipe_table, table_to_csv, table_to_html, table_to_markdown};

#[derive(Default)]
pub struct MarkdownExtractor;

impl Extractor for MarkdownExtractor {
    fn name(&self) -> &'static str {
        "markdown_extractor"
    }

    fn extract(
        &self,
        input_path: &Path,
        classification: &FileClassification,
    ) -> anyhow::Result<crate::model::DocumentModel> {
        let markdown = fs::read_to_string(input_path)
            .with_context(|| format!("failed to read markdown file: {}", input_path.display()))?;

        let mut model = base_document_model(
            classification,
            DocumentFormat::Md,
            ContentMode::Digital,
            PageType::DocumentPage,
        );

        let mut html_output = String::new();
        let parser = Parser::new_ext(&markdown, Options::all());
        html::push_html(&mut html_output, parser);

        let mut elements = Vec::new();
        let mut order = 1_u32;
        let mut y = 20.0_f32;
        let mut in_code_block = false;
        let mut code_lines: Vec<String> = Vec::new();

        for line in markdown.lines() {
            let trimmed = line.trim_end();

            if trimmed.starts_with("```") {
                if in_code_block {
                    let code = code_lines.join("\n");
                    elements.push(make_element(
                        order,
                        y,
                        ElementType::Code,
                        Some("code"),
                        code.clone(),
                        format!("```\n{}\n```", code),
                        code,
                    ));
                    order += 1;
                    y += 28.0;
                    code_lines.clear();
                }
                in_code_block = !in_code_block;
                continue;
            }

            if in_code_block {
                code_lines.push(trimmed.to_string());
                continue;
            }

            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with('#') {
                elements.push(make_element(
                    order,
                    y,
                    ElementType::Heading,
                    Some("h"),
                    trimmed.trim_start_matches('#').trim().to_string(),
                    trimmed.to_string(),
                    trimmed.to_string(),
                ));
            } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                elements.push(make_element(
                    order,
                    y,
                    ElementType::ListItem,
                    Some("li"),
                    trimmed[2..].trim().to_string(),
                    trimmed.to_string(),
                    trimmed.to_string(),
                ));
            } else if trimmed.starts_with('>') {
                elements.push(make_element(
                    order,
                    y,
                    ElementType::Blockquote,
                    Some("blockquote"),
                    trimmed.trim_start_matches('>').trim().to_string(),
                    trimmed.to_string(),
                    trimmed.to_string(),
                ));
            } else {
                elements.push(make_element(
                    order,
                    y,
                    ElementType::Paragraph,
                    Some("p"),
                    trimmed.to_string(),
                    trimmed.to_string(),
                    trimmed.to_string(),
                ));
            }

            order += 1;
            y += 28.0;
        }

        let lines = markdown.lines().collect::<Vec<_>>();
        let pipe_table = (0..lines.len())
            .find_map(|idx| detect_pipe_table(&lines[idx..]));
        if let Some((cells, rows, columns)) = pipe_table {
            let table_markdown = table_to_markdown(&cells, rows, columns);
            let table_csv = table_to_csv(&cells, rows, columns);
            let table_html = table_to_html(&cells, rows, columns);
            let table_text = cells
                .iter()
                .map(|c| c.text.as_str())
                .collect::<Vec<_>>()
                .join(" | ");

            let mut table = make_element(
                order,
                y,
                ElementType::Table,
                Some("table"),
                table_text.clone(),
                table_markdown.clone(),
                table_markdown.clone(),
            );
            table.role = Some("data_table".to_string());
            table.content["html"] = json!(table_html);
            table.content["csv"] = json!(table_csv);
            table.extra.insert("rows".to_string(), json!(rows));
            table.extra.insert("columns".to_string(), json!(columns));
            table.extra.insert("cells".to_string(), serde_json::to_value(&cells).unwrap_or_else(|_| json!([])));
            table.extra.insert(
                "table_structure".to_string(),
                serde_json::to_value(TableStructure {
                    has_header: true,
                    has_merged_cells: false,
                    orientation: "horizontal".to_string(),
                    extraction_method: "markdown_pipe".to_string(),
                })
                .unwrap_or_else(|_| json!({})),
            );
            elements.push(table);
        }

        let formula_elements = extract_markdown_formulas(&markdown, order, y);
        if !formula_elements.is_empty() {
            elements.extend(formula_elements);
        }

        let page = &mut model.pages[0];
        page.elements = elements;
        page.text = page
            .elements
            .iter()
            .filter_map(|e| e.content.get("text").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join("\n");
        page.markdown = markdown;
        page.html = html_output;

        model.document_profile.has_tables = page
            .elements
            .iter()
            .any(|e| matches!(e.element_type, ElementType::Table));
        model.document_profile.has_images = page.markdown.contains("![");
        model.document_profile.has_formulas = page.markdown.contains("$$");

        update_stats(&mut model);
        model.processing.stages.push(stage("markdown_ast_parse", "pulldown-cmark", 1));
        model.processing.total_duration_ms = Some(1);

        Ok(model)
    }
}

fn extract_markdown_formulas(markdown: &str, start_order: u32, start_y: f32) -> Vec<Element> {
    let mut formulas = Vec::new();
    let mut order = start_order;
    let mut y = start_y;
    let mut in_block = false;
    let mut block_lines = Vec::new();

    for line in markdown.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("$$") {
            if in_block {
                let text = block_lines.join("\n").trim().to_string();
                if !text.is_empty() {
                    formulas.push(make_formula_element(order, y, text.clone(), text));
                    order += 1;
                    y += 28.0;
                }
                block_lines.clear();
                in_block = false;
                continue;
            }

            let tail = trimmed.trim_start_matches("$$").trim().trim_end_matches("$$").trim();
            if !tail.is_empty() && trimmed.ends_with("$$") && trimmed.len() > 4 {
                formulas.push(make_formula_element(order, y, tail.to_string(), tail.to_string()));
                order += 1;
                y += 28.0;
                continue;
            }

            in_block = true;
            block_lines.clear();
            continue;
        }

        if in_block {
            block_lines.push(trimmed.to_string());
            continue;
        }

        if trimmed.starts_with("\\[") && trimmed.ends_with("\\]") && trimmed.len() >= 4 {
            let text = trimmed
                .trim_start_matches("\\[")
                .trim_end_matches("\\]")
                .trim()
                .to_string();
            if !text.is_empty() {
                formulas.push(make_formula_element(order, y, text.clone(), text));
                order += 1;
                y += 28.0;
            }
        }
    }

    formulas
}

fn make_formula_element(order: u32, y: f32, latex: String, raw: String) -> Element {
    let mut element = make_element(
        order,
        y,
        ElementType::Formula,
        Some("formula"),
        latex.clone(),
        format!("$$\n{}\n$$", latex),
        raw,
    );
    element.role = Some("math_formula".to_string());
    element.extra.insert("format".to_string(), json!("latex"));
    element.extra.insert("latex_source".to_string(), json!(latex));
    element
}

fn make_element(
    order: u32,
    y: f32,
    element_type: ElementType,
    tag: Option<&str>,
    text: String,
    markdown: String,
    raw: String,
) -> Element {
    Element {
        element_id: format!("p1_e{}", order),
        element_type,
        tag: tag.map(ToString::to_string),
        role: None,
        reading_order: Some(order),
        global_order: Some(order),
        bbox: Some([40.0, y, 1160.0, y + 24.0]),
        polygon: None,
        content: json!({
            "text": text,
            "html": null,
            "markdown": markdown,
            "normalized_text": text.to_lowercase(),
            "raw": raw,
        }),
        style: empty_style(),
        provenance: provenance("markdown_parser", "markdown_native_extraction", "path", "line"),
        confidence: default_confidence(),
        warnings: vec![],
        extra: HashMap::new(),
    }
}
