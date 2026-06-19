use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::Context;
use scraper::{Html, Selector};
use serde_json::json;

use crate::classifier::FileClassification;
use crate::extractors::{
    base_document_model, default_confidence, empty_style, provenance, stage, update_stats,
};
use crate::model::{ContentMode, DocumentFormat, Element, ElementType, PageType};
use crate::router::Extractor;
use crate::tables::{TableCell, TableStructure, table_to_csv, table_to_html, table_to_markdown};

#[derive(Default)]
pub struct HtmlExtractor;

impl Extractor for HtmlExtractor {
    fn name(&self) -> &'static str {
        "html_extractor"
    }

    fn extract(&self, input_path: &Path, classification: &FileClassification) -> anyhow::Result<crate::model::DocumentModel> {
        let content = fs::read_to_string(input_path)
            .with_context(|| format!("failed to read html file: {}", input_path.display()))?;

        let mut model = base_document_model(
            classification,
            DocumentFormat::Html,
            ContentMode::Digital,
            PageType::DocumentPage,
        );

        let document = Html::parse_document(&content);

        let mut elements = Vec::new();
        let mut reading_order = 1_u32;
        let mut y = 20.0_f32;

        let push_node = |elements: &mut Vec<Element>,
                         reading_order: u32,
                         y: f32,
                         element_type: ElementType,
                         tag: &str,
                         text: String,
                         markdown: String,
                         raw: String| {
            elements.push(Element {
                element_id: format!("p1_e{}", reading_order),
                element_type,
                tag: Some(tag.to_string()),
                role: None,
                reading_order: Some(reading_order),
                global_order: Some(reading_order),
                bbox: Some([40.0, y, 1160.0, y + 24.0]),
                polygon: None,
                content: json!({
                    "text": text,
                    "html": raw,
                    "markdown": markdown,
                    "normalized_text": text.to_lowercase(),
                    "raw": raw,
                }),
                style: empty_style(),
                provenance: provenance("html_dom_parser", "html_extraction", "path", "inline_dom"),
                confidence: default_confidence(),
                warnings: vec![],
                extra: HashMap::new(),
            });
        };

        let selectors = vec![
            ("h1", ElementType::Heading),
            ("h2", ElementType::Heading),
            ("h3", ElementType::Heading),
            ("h4", ElementType::Heading),
            ("h5", ElementType::Heading),
            ("h6", ElementType::Heading),
            ("p", ElementType::Paragraph),
            ("blockquote", ElementType::Blockquote),
            ("pre", ElementType::Code),
            ("code", ElementType::Code),
            ("ul", ElementType::List),
            ("ol", ElementType::List),
            ("li", ElementType::ListItem),
        ];

        for (css, element_type) in selectors {
            let selector = Selector::parse(css)
                .map_err(|_| anyhow::anyhow!("failed to parse selector: {css}"))?;
            for node in document.select(&selector) {
                let text = node.text().collect::<Vec<_>>().join(" ").trim().to_string();
                if text.is_empty() {
                    continue;
                }

                let markdown = if css.starts_with('h') && css.len() == 2 {
                    let level = css.chars().nth(1).unwrap_or('1').to_digit(10).unwrap_or(1) as usize;
                    format!("{} {}", "#".repeat(level), text)
                } else if css == "li" {
                    format!("- {text}")
                } else if css == "blockquote" {
                    format!("> {text}")
                } else if css == "pre" || css == "code" {
                    format!("```\n{text}\n```")
                } else {
                    text.clone()
                };

                push_node(
                    &mut elements,
                    reading_order,
                    y,
                    element_type.clone(),
                    css,
                    text,
                    markdown,
                    node.html(),
                );
                reading_order += 1;
                y += 28.0;
            }
        }

        if let Ok(table_selector) = Selector::parse("table") {
            if let (Ok(row_selector), Ok(cell_selector)) = (Selector::parse("tr"), Selector::parse("th, td")) {
                for table in document.select(&table_selector) {
                    let mut cells = Vec::new();
                    let mut row_count = 0_usize;
                    let mut col_count = 0_usize;
                    let mut has_header = false;
                    let mut has_merged = false;

                    for (r_idx, row) in table.select(&row_selector).enumerate() {
                        row_count = row_count.max(r_idx + 1);
                        let mut c_idx = 0_usize;
                        for cell in row.select(&cell_selector) {
                            let tag = cell.value().name();
                            let text = cell.text().collect::<Vec<_>>().join(" ").trim().to_string();
                            let rowspan = cell
                                .value()
                                .attr("rowspan")
                                .and_then(|v| v.parse::<usize>().ok())
                                .unwrap_or(1);
                            let colspan = cell
                                .value()
                                .attr("colspan")
                                .and_then(|v| v.parse::<usize>().ok())
                                .unwrap_or(1);
                            if rowspan > 1 || colspan > 1 {
                                has_merged = true;
                            }
                            if tag == "th" {
                                has_header = true;
                            }

                            cells.push(TableCell {
                                row: r_idx,
                                column: c_idx,
                                rowspan,
                                colspan,
                                bbox: None,
                                text,
                                html: Some(cell.html()),
                                markdown: None,
                                formula: None,
                                is_header: tag == "th",
                                confidence: None,
                            });

                            c_idx += 1;
                        }
                        col_count = col_count.max(c_idx);
                    }

                    if row_count == 0 || col_count == 0 {
                        continue;
                    }

                    let markdown = table_to_markdown(&cells, row_count, col_count);
                    let csv = table_to_csv(&cells, row_count, col_count);
                    let html = table_to_html(&cells, row_count, col_count);
                    let text = cells
                        .iter()
                        .map(|c| c.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" | ");

                    let mut element = Element {
                        element_id: format!("p1_table_{}", reading_order),
                        element_type: ElementType::Table,
                        tag: Some("table".to_string()),
                        role: Some("data_table".to_string()),
                        reading_order: Some(reading_order),
                        global_order: Some(reading_order),
                        bbox: Some([40.0, y, 1160.0, y + 24.0]),
                        polygon: None,
                        content: json!({
                            "text": text,
                            "html": html,
                            "markdown": markdown,
                            "csv": csv,
                            "normalized_text": text.to_lowercase(),
                            "raw": table.html(),
                        }),
                        style: empty_style(),
                        provenance: json!({
                            "method": "native",
                            "tool": "html_dom_parser",
                            "stage": "html_table_extraction"
                        }),
                        confidence: default_confidence(),
                        warnings: vec![],
                        extra: HashMap::new(),
                    };

                    if has_merged {
                        element.warnings.push(crate::model::Diagnostic {
                            code: "COMPLEX_TABLE_MARKDOWN_APPROXIMATION".to_string(),
                            severity: "warning".to_string(),
                            scope: "element".to_string(),
                            page_number: Some(1),
                            element_id: Some(element.element_id.clone()),
                            message: "Markdown таблица сформирована приблизительно для сложной структуры.".to_string(),
                            recoverable: true,
                            extra: HashMap::new(),
                        });
                    }

                    element.extra.insert("rows".to_string(), json!(row_count));
                    element.extra.insert("columns".to_string(), json!(col_count));
                    element.extra.insert("cells".to_string(), serde_json::to_value(&cells).unwrap_or_else(|_| json!([])));
                    element.extra.insert(
                        "table_structure".to_string(),
                        serde_json::to_value(TableStructure {
                            has_header,
                            has_merged_cells: has_merged,
                            orientation: "horizontal".to_string(),
                            extraction_method: "html_native".to_string(),
                        })
                        .unwrap_or_else(|_| json!({})),
                    );

                    elements.push(element);
                    reading_order += 1;
                    y += 28.0;
                }
            }
        }

        if let Ok(math_selector) = Selector::parse("math") {
            for node in document.select(&math_selector) {
                let text = node.text().collect::<Vec<_>>().join(" ").trim().to_string();
                if text.is_empty() {
                    continue;
                }

                let mut element = Element {
                    element_id: format!("p1_formula_{}", reading_order),
                    element_type: ElementType::Formula,
                    tag: Some("math".to_string()),
                    role: Some("math_formula".to_string()),
                    reading_order: Some(reading_order),
                    global_order: Some(reading_order),
                    bbox: Some([40.0, y, 1160.0, y + 24.0]),
                    polygon: None,
                    content: json!({
                        "text": text,
                        "html": node.html(),
                        "markdown": format!("$$ {} $$", node.text().collect::<Vec<_>>().join(" ").trim()),
                        "normalized_text": node.text().collect::<Vec<_>>().join(" ").trim().to_string(),
                        "raw": node.html(),
                    }),
                    style: empty_style(),
                    provenance: json!({
                        "method": "native",
                        "tool": "html_dom_parser",
                        "stage": "html_formula_extraction"
                    }),
                    confidence: default_confidence(),
                    warnings: vec![],
                    extra: HashMap::new(),
                };
                element.extra.insert("format".to_string(), json!("mathml"));
                elements.push(element);
                reading_order += 1;
                y += 28.0;
            }
        }

        // Stable logical order when selectors capture the same node multiple times.
        elements.sort_by_key(|e| e.reading_order.unwrap_or(0));

        let page = &mut model.pages[0];
        page.elements = elements;
        page.text = page
            .elements
            .iter()
            .filter_map(|e| e.content.get("text").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join("\n");
        page.markdown = page
            .elements
            .iter()
            .filter_map(|e| e.content.get("markdown").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join("\n\n");
        page.html = content;

        model.document_profile.has_images = Selector::parse("img")
            .ok()
            .map(|s| document.select(&s).next().is_some())
            .unwrap_or(false);
        model.document_profile.has_tables = Selector::parse("table")
            .ok()
            .map(|s| document.select(&s).next().is_some())
            .unwrap_or(false);
        model.document_profile.has_formulas = Selector::parse("math")
            .ok()
            .map(|s| document.select(&s).next().is_some())
            .unwrap_or(false);

        update_stats(&mut model);
        model.processing.stages.push(stage("html_parse", "scraper", 1));
        model.processing.total_duration_ms = Some(1);

        Ok(model)
    }
}
