use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::merge::text_similarity;
use crate::model::{DocumentModel, ElementType};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QualityReport {
    pub document_id: String,
    pub format: String,
    pub language: String,
    pub pages: usize,
    pub elements: usize,
    pub chars: usize,
    pub words: usize,
    pub tables: usize,
    pub images: usize,
    pub formulas: usize,
    pub ocr_elements: usize,
    pub warnings: usize,
    pub errors: usize,
    pub empty_pages: usize,
    pub duplicate_text_ratio: f32,
    pub low_confidence_ocr_ratio: f32,
    pub chunk_count: usize,
}

impl QualityReport {
    pub fn from_model(model: &DocumentModel) -> Self {
        let mut elements = 0_usize;
        let mut tables = 0_usize;
        let mut images = 0_usize;
        let mut formulas = 0_usize;
        let mut ocr_elements = 0_usize;
        let mut empty_pages = 0_usize;
        let mut warnings = model.warnings.len();

        let mut total_ocr = 0_usize;
        let mut low_conf_ocr = 0_usize;

        for page in &model.pages {
            if page.text.trim().is_empty() {
                empty_pages = empty_pages.saturating_add(1);
            }

            warnings = warnings.saturating_add(page.warnings.len());
            for element in &page.elements {
                elements = elements.saturating_add(1);
                warnings = warnings.saturating_add(element.warnings.len());
                match element.element_type {
                    ElementType::Table => tables = tables.saturating_add(1),
                    ElementType::Image | ElementType::PageImage => {
                        images = images.saturating_add(1)
                    }
                    ElementType::Formula => formulas = formulas.saturating_add(1),
                    ElementType::TextOcr => {
                        ocr_elements = ocr_elements.saturating_add(1);
                        total_ocr = total_ocr.saturating_add(1);
                        let confidence = element
                            .confidence
                            .get("overall")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(1.0);
                        if confidence < 0.5 {
                            low_conf_ocr = low_conf_ocr.saturating_add(1);
                        }
                    }
                    _ => {}
                }
            }
        }

        let texts = collect_text_blocks(model);
        let chars = texts.iter().map(|v| v.chars().count()).sum::<usize>();
        let words = texts
            .iter()
            .map(|v| v.split_whitespace().count())
            .sum::<usize>();

        let duplicate_text_ratio = duplicate_text_ratio(&texts);
        let low_confidence_ocr_ratio = if total_ocr == 0 {
            0.0
        } else {
            low_conf_ocr as f32 / total_ocr as f32
        };

        Self {
            document_id: model.document_id.clone(),
            format: serde_json::to_value(&model.document_profile.format)
                .ok()
                .and_then(|v| v.as_str().map(ToOwned::to_owned))
                .unwrap_or_else(|| "unknown".to_string()),
            language: model
                .document_profile
                .languages
                .first()
                .cloned()
                .unwrap_or_else(|| "ru".to_string()),
            pages: model.pages.len(),
            elements,
            chars,
            words,
            tables,
            images,
            formulas,
            ocr_elements,
            warnings,
            errors: model.errors.len(),
            empty_pages,
            duplicate_text_ratio,
            low_confidence_ocr_ratio,
            chunk_count: model.chunks.len(),
        }
    }
}

pub fn generate_quality_report_from_model_path(model_path: &Path) -> anyhow::Result<QualityReport> {
    let bytes = std::fs::read(model_path)?;
    let model: DocumentModel = serde_json::from_slice(&bytes)?;
    Ok(QualityReport::from_model(&model))
}

pub fn write_quality_report(
    report: &QualityReport,
    output_dir: &Path,
) -> anyhow::Result<(PathBuf, PathBuf)> {
    std::fs::create_dir_all(output_dir)?;

    let json_path = output_dir.join("quality_report.json");
    let md_path = output_dir.join("quality_report.md");

    std::fs::write(&json_path, serde_json::to_vec_pretty(report)?)?;

    let mut md = String::new();
    let _ = writeln!(md, "# Отчет качества извлечения");
    let _ = writeln!(md);
    let _ = writeln!(md, "- Документ: {}", report.document_id);
    let _ = writeln!(md, "- Формат: {}", report.format);
    let _ = writeln!(md, "- Язык: {}", report.language);
    let _ = writeln!(md, "- Страниц: {}", report.pages);
    let _ = writeln!(md, "- Элементов: {}", report.elements);
    let _ = writeln!(md, "- Символов: {}", report.chars);
    let _ = writeln!(md, "- Слов: {}", report.words);
    let _ = writeln!(md, "- Таблиц: {}", report.tables);
    let _ = writeln!(md, "- Изображений: {}", report.images);
    let _ = writeln!(md, "- Формул: {}", report.formulas);
    let _ = writeln!(md, "- OCR-элементов: {}", report.ocr_elements);
    let _ = writeln!(md, "- Предупреждений: {}", report.warnings);
    let _ = writeln!(md, "- Ошибок: {}", report.errors);
    let _ = writeln!(md, "- Пустых страниц: {}", report.empty_pages);
    let _ = writeln!(
        md,
        "- Коэффициент дублирования текста: {:.4}",
        report.duplicate_text_ratio
    );
    let _ = writeln!(
        md,
        "- Доля OCR с низкой уверенностью: {:.4}",
        report.low_confidence_ocr_ratio
    );
    let _ = writeln!(md, "- Количество чанков: {}", report.chunk_count);

    std::fs::write(&md_path, md)?;
    Ok((json_path, md_path))
}

fn collect_text_blocks(model: &DocumentModel) -> Vec<String> {
    let mut blocks = Vec::new();

    for page in &model.pages {
        if !page.text.trim().is_empty() {
            blocks.push(page.text.trim().to_string());
        }
        for element in &page.elements {
            if let Some(text) = element.content.get("text").and_then(|v| v.as_str()) {
                if !text.trim().is_empty() {
                    blocks.push(text.trim().to_string());
                }
            }
        }
    }

    blocks
}

fn duplicate_text_ratio(blocks: &[String]) -> f32 {
    if blocks.is_empty() {
        return 0.0;
    }

    let normalized = blocks
        .iter()
        .map(|v| normalize_block(v))
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>();
    if normalized.is_empty() {
        return 0.0;
    }

    let total_chars = normalized.iter().map(|v| v.chars().count()).sum::<usize>();
    if total_chars == 0 {
        return 0.0;
    }

    let mut exact_counts: HashMap<String, usize> = HashMap::new();
    for text in &normalized {
        *exact_counts.entry(text.clone()).or_insert(0) += 1;
    }

    let mut duplicate_chars = 0_usize;
    for (text, count) in exact_counts {
        if count > 1 {
            duplicate_chars = duplicate_chars.saturating_add(text.chars().count() * (count - 1));
        }
    }

    // Near-duplicate estimate for non-identical repeated blocks.
    for i in 0..normalized.len() {
        for j in (i + 1)..normalized.len() {
            let a = &normalized[i];
            let b = &normalized[j];
            if a == b {
                continue;
            }
            if text_similarity(a, b) >= 0.92 {
                duplicate_chars = duplicate_chars.saturating_add(a.chars().count().min(b.chars().count()) / 2);
            }
        }
    }

    (duplicate_chars as f32 / total_chars as f32).clamp(0.0, 1.0)
}

fn normalize_block(input: &str) -> String {
    input
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
