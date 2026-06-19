use serde::{Deserialize, Serialize};

use crate::model::{DocumentModel, ElementType};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegressionAssertions {
    pub min_pages: Option<usize>,
    pub min_elements: Option<usize>,
    #[serde(default)]
    pub must_contain_text: Vec<String>,
    #[serde(default)]
    pub must_have_element_types: Vec<String>,
    #[serde(default)]
    pub must_have_chunks: bool,
    pub must_have_tables: Option<bool>,
    pub must_have_ocr: Option<bool>,
    pub max_errors: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegressionTolerance {
    #[serde(default)]
    pub ignore_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegressionExpectation {
    #[serde(flatten)]
    pub assertions: RegressionAssertions,
}

pub fn evaluate_assertions(
    assertions: &RegressionAssertions,
    model: &DocumentModel,
) -> anyhow::Result<()> {
    if let Some(min_pages) = assertions.min_pages {
        if model.pages.len() < min_pages {
            return Err(anyhow::anyhow!(
                "REGRESSION_ASSERTION_FAILED: ожидалось минимум {} страниц, получено {}",
                min_pages,
                model.pages.len()
            ));
        }
    }

    let element_count = model.pages.iter().map(|p| p.elements.len()).sum::<usize>();
    if let Some(min_elements) = assertions.min_elements {
        if element_count < min_elements {
            return Err(anyhow::anyhow!(
                "REGRESSION_ASSERTION_FAILED: ожидалось минимум {} элементов, получено {}",
                min_elements,
                element_count
            ));
        }
    }

    for needle in &assertions.must_contain_text {
        let found = model.pages.iter().any(|page| {
            page.text.contains(needle)
                || page
                    .elements
                    .iter()
                    .any(|e| e.content.get("text").and_then(|v| v.as_str()).map(|t| t.contains(needle)).unwrap_or(false))
        });

        if !found {
            return Err(anyhow::anyhow!(
                "REGRESSION_ASSERTION_FAILED: текст '{}' не найден",
                needle
            ));
        }
    }

    for expected_type in &assertions.must_have_element_types {
        let found = model.pages.iter().flat_map(|p| p.elements.iter()).any(|element| {
            element_type_name(element.element_type.clone()) == expected_type.to_ascii_lowercase()
        });

        if !found {
            return Err(anyhow::anyhow!(
                "REGRESSION_ASSERTION_FAILED: отсутствует элемент типа '{}'",
                expected_type
            ));
        }
    }

    if assertions.must_have_chunks && model.chunks.is_empty() {
        return Err(anyhow::anyhow!(
            "REGRESSION_ASSERTION_FAILED: ожидались чанки, но список chunks пуст"
        ));
    }

    if let Some(must_have_tables) = assertions.must_have_tables {
        let has_tables = model.pages.iter().flat_map(|p| p.elements.iter()).any(|e| {
            matches!(e.element_type, ElementType::Table)
        });
        if has_tables != must_have_tables {
            return Err(anyhow::anyhow!(
                "REGRESSION_ASSERTION_FAILED: проверка таблиц не прошла"
            ));
        }
    }

    if let Some(must_have_ocr) = assertions.must_have_ocr {
        let has_ocr = model.pages.iter().flat_map(|p| p.elements.iter()).any(|e| {
            matches!(e.element_type, ElementType::TextOcr)
        });
        if has_ocr != must_have_ocr {
            return Err(anyhow::anyhow!(
                "REGRESSION_ASSERTION_FAILED: проверка OCR-элементов не прошла"
            ));
        }
    }

    if let Some(max_errors) = assertions.max_errors {
        if model.errors.len() > max_errors {
            return Err(anyhow::anyhow!(
                "REGRESSION_ASSERTION_FAILED: ошибок больше лимита ({} > {})",
                model.errors.len(),
                max_errors
            ));
        }
    }

    Ok(())
}

fn element_type_name(kind: ElementType) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "unknown".to_string())
}
