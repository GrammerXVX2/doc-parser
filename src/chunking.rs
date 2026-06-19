use serde_json::json;

use crate::model::{Chunk, DocumentModel, Element, ElementType};
use crate::tables::{TableCell, TableLinearizationOptions, linearize_cells};

#[derive(Debug, Clone)]
pub struct SemanticChunker {
    pub max_token_estimate: usize,
}

impl Default for SemanticChunker {
    fn default() -> Self {
        Self {
            max_token_estimate: 1_000,
        }
    }
}

impl SemanticChunker {
    pub fn chunk_document(&self, document: &DocumentModel) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let mut current_title: Option<String> = None;
        let mut current_section_path: Vec<String> = Vec::new();
        let mut current_text = String::new();
        let mut current_markdown = String::new();
        let mut current_element_ids: Vec<String> = Vec::new();
        let mut current_page_start = 1_u32;
        let mut current_page_end = 1_u32;

        for page in &document.pages {
            for element in &page.elements {
                if element
                    .extra
                    .get("exclude_from_chunks")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    continue;
                }

                if matches!(element.element_type, ElementType::Table) {
                    if !current_element_ids.is_empty() {
                        chunks.push(build_chunk(
                            chunks.len() + 1,
                            current_title.clone(),
                            current_section_path.clone(),
                            current_page_start,
                            current_page_end,
                            std::mem::take(&mut current_element_ids),
                            std::mem::take(&mut current_text),
                            std::mem::take(&mut current_markdown),
                        ));
                    }

                    chunks.push(build_table_chunk(chunks.len() + 1, page.page_number, element));
                    current_page_start = page.page_number;
                    current_page_end = page.page_number;
                    continue;
                }

                let text = element_text(element);
                let markdown = element_markdown(element);

                let is_heading = matches!(element.element_type, ElementType::Heading);
                let would_overflow = estimate_tokens(current_text.len() + text.len()) > self.max_token_estimate;

                if is_heading && !current_element_ids.is_empty() {
                    chunks.push(build_chunk(
                        chunks.len() + 1,
                        current_title.clone(),
                        current_section_path.clone(),
                        current_page_start,
                        current_page_end,
                        std::mem::take(&mut current_element_ids),
                        std::mem::take(&mut current_text),
                        std::mem::take(&mut current_markdown),
                    ));
                    current_page_start = page.page_number;
                }

                if would_overflow && !current_element_ids.is_empty() {
                    chunks.push(build_chunk(
                        chunks.len() + 1,
                        current_title.clone(),
                        current_section_path.clone(),
                        current_page_start,
                        current_page_end,
                        std::mem::take(&mut current_element_ids),
                        std::mem::take(&mut current_text),
                        std::mem::take(&mut current_markdown),
                    ));
                    current_page_start = page.page_number;
                }

                if is_heading {
                    current_title = Some(text.clone());
                    current_section_path = vec![text.clone()];
                }

                current_page_end = page.page_number;
                current_element_ids.push(element.element_id.clone());
                if !text.is_empty() {
                    if !current_text.is_empty() {
                        current_text.push('\n');
                    }
                    current_text.push_str(&text);
                }
                if !markdown.is_empty() {
                    if !current_markdown.is_empty() {
                        current_markdown.push_str("\n\n");
                    }
                    current_markdown.push_str(&markdown);
                }
            }
        }

        if !current_element_ids.is_empty() {
            chunks.push(build_chunk(
                chunks.len() + 1,
                current_title,
                current_section_path,
                current_page_start,
                current_page_end,
                current_element_ids,
                current_text,
                current_markdown,
            ));
        }

        chunks
    }
}

fn build_chunk(
    index: usize,
    title: Option<String>,
    section_path: Vec<String>,
    page_start: u32,
    page_end: u32,
    element_ids: Vec<String>,
    text: String,
    markdown: String,
) -> Chunk {
    let chunk_type = if text.contains('|') {
        "table".to_string()
    } else if text.contains("```") {
        "code".to_string()
    } else {
        "section".to_string()
    };
    let contains_table = chunk_type == "table" || markdown.contains('|');
    let contains_formula = markdown.contains("$$") || text.contains('=') && text.contains('^');
    let contains_image = markdown.contains("![");
    let contains_ocr = text.to_ascii_lowercase().contains("invoice")
        && text.to_ascii_lowercase().contains("total");
    let contains_code = chunk_type == "code" || markdown.contains("```");

    Chunk {
        chunk_id: format!("chunk_{}", index),
        chunk_type,
        title,
        section_path,
        page_start,
        page_end,
        element_ids,
        token_estimate: estimate_tokens(text.len()) as u32,
        text,
        markdown,
        metadata: json!({
            "language": "ru",
            "contains_table": contains_table,
            "contains_image": contains_image,
            "contains_formula": contains_formula,
            "contains_code": contains_code,
            "contains_ocr": contains_ocr
        }),
        extra: Default::default(),
    }
}

fn build_table_chunk(index: usize, page_number: u32, element: &Element) -> Chunk {
    let markdown = element_markdown(element);
    let rows = element
        .extra
        .get("rows")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(0);
    let columns = element
        .extra
        .get("columns")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(0);
    let cells = element
        .extra
        .get("cells")
        .and_then(|v| serde_json::from_value::<Vec<TableCell>>(v.clone()).ok())
        .unwrap_or_default();

    let linearized = linearize_cells(
        &cells,
        rows,
        columns,
        TableLinearizationOptions {
            max_rows_per_chunk: 20,
            language: "ru".to_string(),
        },
    );
    let text = if linearized.is_empty() {
        element_text(element)
    } else {
        linearized
            .iter()
            .map(|chunk| chunk.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    };

    Chunk {
        chunk_id: format!("chunk_table_{}", index),
        chunk_type: "table".to_string(),
        title: Some("Таблица".to_string()),
        section_path: vec![],
        page_start: page_number,
        page_end: page_number,
        element_ids: vec![element.element_id.clone()],
        token_estimate: estimate_tokens(text.len()) as u32,
        text,
        markdown,
        metadata: json!({
            "language": "ru",
            "contains_table": true,
            "contains_image": false,
            "contains_formula": element
                .content
                .get("text")
                .and_then(|v| v.as_str())
                .map(|v| v.contains('=') || v.contains("SUM("))
                .unwrap_or(false),
            "contains_code": false,
            "contains_ocr": false
        }),
        extra: Default::default(),
    }
}

fn estimate_tokens(char_count: usize) -> usize {
    char_count / 4 + 1
}

fn element_text(element: &Element) -> String {
    element
        .content
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn element_markdown(element: &Element) -> String {
    element
        .content
        .get("markdown")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}
