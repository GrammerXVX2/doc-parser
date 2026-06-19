use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::Context;
use serde_json::json;

use crate::classifier::FileClassification;
use crate::extractors::{
    base_document_model, default_confidence, empty_style, provenance, stage, update_stats,
};
use crate::model::{ContentMode, DocumentFormat, Element, ElementType, PageType};
use crate::router::Extractor;
use crate::tables::{TableStructure, detect_pipe_table, detect_tsv_table, table_to_csv, table_to_html, table_to_markdown};

#[derive(Default)]
pub struct TxtExtractor;

impl Extractor for TxtExtractor {
    fn name(&self) -> &'static str {
        "txt_extractor"
    }

    fn extract(
        &self,
        input_path: &Path,
        classification: &FileClassification,
    ) -> anyhow::Result<crate::model::DocumentModel> {
        let bytes = fs::read(input_path)
            .with_context(|| format!("failed to read text file: {}", input_path.display()))?;
        let decoded = String::from_utf8_lossy(&bytes).to_string();
        let normalized = decoded.replace("\r\n", "\n").replace('\r', "\n");

        let mut model = base_document_model(
            classification,
            DocumentFormat::Txt,
            ContentMode::PlainText,
            PageType::SyntheticTextPage,
        );

        let mut elements = Vec::new();
        let mut order = 1_u32;
        let mut y = 0.0_f32;

        let lines = normalized.lines().collect::<Vec<_>>();
        let pipe_table = (0..lines.len())
            .find_map(|idx| detect_pipe_table(&lines[idx..]));
        if let Some((cells, rows, columns)) = pipe_table {
            let markdown = table_to_markdown(&cells, rows, columns);
            let csv = table_to_csv(&cells, rows, columns);
            let html = table_to_html(&cells, rows, columns);
            let text = cells
                .iter()
                .map(|c| c.text.as_str())
                .collect::<Vec<_>>()
                .join(" | ");

            let mut table = Element {
                element_id: format!("p1_e{}", order),
                element_type: ElementType::Table,
                tag: Some("txt_table".to_string()),
                role: Some("data_table".to_string()),
                reading_order: Some(order),
                global_order: Some(order),
                bbox: Some([0.0, y, 120.0, y + 1.0]),
                polygon: None,
                content: json!({
                    "text": text,
                    "html": html,
                    "markdown": markdown,
                    "csv": csv,
                    "normalized_text": text.to_lowercase(),
                    "raw": markdown,
                }),
                style: empty_style(),
                provenance: provenance("txt_parser", "table_detection", "line", &order.to_string()),
                confidence: default_confidence(),
                warnings: vec![],
                extra: HashMap::new(),
            };
            table.extra.insert("rows".to_string(), json!(rows));
            table.extra.insert("columns".to_string(), json!(columns));
            table.extra.insert("cells".to_string(), serde_json::to_value(&cells).unwrap_or_else(|_| json!([])));
            table.extra.insert(
                "table_structure".to_string(),
                serde_json::to_value(TableStructure {
                    has_header: true,
                    has_merged_cells: false,
                    orientation: "horizontal".to_string(),
                    extraction_method: "txt_pipe".to_string(),
                })
                .unwrap_or_else(|_| json!({})),
            );
            elements.push(table);
            order += 1;
            y += 1.0;
        }

        let tsv_table = (0..lines.len())
            .find_map(|idx| detect_tsv_table(&lines[idx..]));
        if let Some((cells, rows, columns)) = tsv_table {
            let markdown = table_to_markdown(&cells, rows, columns);
            let csv = table_to_csv(&cells, rows, columns);
            let html = table_to_html(&cells, rows, columns);
            let text = cells
                .iter()
                .map(|c| c.text.as_str())
                .collect::<Vec<_>>()
                .join(" | ");

            let mut table = Element {
                element_id: format!("p1_e{}", order),
                element_type: ElementType::Table,
                tag: Some("txt_table".to_string()),
                role: Some("data_table".to_string()),
                reading_order: Some(order),
                global_order: Some(order),
                bbox: Some([0.0, y, 120.0, y + 1.0]),
                polygon: None,
                content: json!({
                    "text": text,
                    "html": html,
                    "markdown": markdown,
                    "csv": csv,
                    "normalized_text": text.to_lowercase(),
                    "raw": markdown,
                }),
                style: empty_style(),
                provenance: provenance("txt_parser", "table_detection", "line", &order.to_string()),
                confidence: default_confidence(),
                warnings: vec![],
                extra: HashMap::new(),
            };
            table.extra.insert("rows".to_string(), json!(rows));
            table.extra.insert("columns".to_string(), json!(columns));
            table.extra.insert("cells".to_string(), serde_json::to_value(&cells).unwrap_or_else(|_| json!([])));
            table.extra.insert(
                "table_structure".to_string(),
                serde_json::to_value(TableStructure {
                    has_header: true,
                    has_merged_cells: false,
                    orientation: "horizontal".to_string(),
                    extraction_method: "txt_tsv".to_string(),
                })
                .unwrap_or_else(|_| json!({})),
            );
            elements.push(table);
            order += 1;
            y += 1.0;
        }

        for line in normalized.lines() {
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                continue;
            }

            let (element_type, role, markdown) = if looks_like_heading(trimmed) {
                (
                    ElementType::Heading,
                    Some("detected_heading".to_string()),
                    format!("# {}", trimmed.trim()),
                )
            } else if looks_like_list_item(trimmed) {
                (
                    ElementType::ListItem,
                    Some("detected_list_item".to_string()),
                    format!("- {}", trimmed.trim_start_matches(['-', '*', ' ']).trim()),
                )
            } else if looks_like_code(trimmed) {
                (
                    ElementType::Code,
                    Some("detected_code".to_string()),
                    format!("```\n{}\n```", trimmed),
                )
            } else {
                (ElementType::Paragraph, Some("paragraph".to_string()), trimmed.to_string())
            };

            elements.push(Element {
                element_id: format!("p1_e{}", order),
                element_type,
                tag: None,
                role,
                reading_order: Some(order),
                global_order: Some(order),
                bbox: Some([0.0, y, 120.0, y + 1.0]),
                polygon: None,
                content: json!({
                    "text": trimmed,
                    "html": null,
                    "markdown": markdown,
                    "normalized_text": trimmed.to_lowercase(),
                    "raw": trimmed,
                }),
                style: empty_style(),
                provenance: provenance("txt_parser", "structure_detection", "line", &order.to_string()),
                confidence: default_confidence(),
                warnings: vec![],
                extra: HashMap::new(),
            });

            order += 1;
            y += 1.0;
        }

        let page = &mut model.pages[0];
        page.elements = elements;
        page.text = normalized.clone();
        page.markdown = normalized
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        page.html = format!("<pre>{}</pre>", html_escape(&normalized));

        update_stats(&mut model);
        model.document_profile.has_tables = model
            .pages
            .iter()
            .flat_map(|p| p.elements.iter())
            .any(|e| matches!(e.element_type, ElementType::Table));
        model.processing.stages.push(stage("encoding_detection", "utf8_lossy", 1));
        model.processing.stages.push(stage("structure_detection", "line_rules", 1));
        model.processing.total_duration_ms = Some(2);

        Ok(model)
    }
}

fn looks_like_heading(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.starts_with('#') {
        return true;
    }

    if trimmed.is_empty() || trimmed.len() > 80 {
        return false;
    }

    let letters = trimmed
        .chars()
        .filter(|c| c.is_alphabetic())
        .collect::<Vec<_>>();
    if letters.is_empty() {
        return false;
    }

    let has_upper = letters.iter().any(|c| c.is_uppercase());
    let has_lower = letters.iter().any(|c| c.is_lowercase());
    has_upper && !has_lower
}

fn looks_like_list_item(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("- ") || trimmed.starts_with("* ") || starts_with_ordered_list(trimmed)
}

fn starts_with_ordered_list(line: &str) -> bool {
    let mut seen_digit = false;
    for c in line.chars() {
        if c.is_ascii_digit() {
            seen_digit = true;
            continue;
        }
        if seen_digit && (c == '.' || c == ')') {
            return true;
        }
        break;
    }
    false
}

fn looks_like_code(line: &str) -> bool {
    line.starts_with("    ") || line.contains("{") || line.contains("}") || line.contains(";")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
