use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Context;
use calamine::{Reader, open_workbook_auto};
use serde_json::json;

use crate::assets::{AssetStore, AssetType, LocalAssetStore};
use crate::classifier::FileClassification;
use crate::config::PipelineConfig;
use crate::extractors::{
    base_document_model, default_confidence, empty_style, stage, update_stats, warning,
};
use crate::model::{
    ContentMode, Diagnostic, DocumentFormat, Element, ElementType, Page, PageProfile, PageType,
    ProcessingStatus,
};
use crate::office::media::{content_type_for_media_path, list_media_entries};
use crate::office::ooxml::OoxmlPackage;
use crate::router::Extractor;
use crate::runtime::output_root_dir;
use crate::tables::{
    TableCell, TableLinearizationOptions, TableStructure, detect_xlsx_table_ranges, linearize_cells,
    table_to_csv, table_to_html, table_to_markdown,
};
use crate::utils::office_text::normalize_office_text;

#[derive(Default)]
pub struct XlsxExtractor;

#[derive(Debug, Clone)]
struct XlsxOptions {
    enabled: bool,
    extract_sheets: bool,
    extract_cells: bool,
    extract_formulas: bool,
    extract_comments: bool,
    extract_images: bool,
    detect_tables: bool,
    used_range_only: bool,
    max_rows_per_table_element: usize,
    max_rows_per_linearized_chunk: usize,
}

impl Default for XlsxOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            extract_sheets: true,
            extract_cells: true,
            extract_formulas: true,
            extract_comments: true,
            extract_images: true,
            detect_tables: true,
            used_range_only: true,
            max_rows_per_table_element: 200,
            max_rows_per_linearized_chunk: 20,
        }
    }
}

impl Extractor for XlsxExtractor {
    fn name(&self) -> &'static str {
        "xlsx_extractor"
    }

    fn extract(
        &self,
        input_path: &Path,
        classification: &FileClassification,
    ) -> anyhow::Result<crate::model::DocumentModel> {
        let mut model = base_document_model(
            classification,
            DocumentFormat::Xlsx,
            ContentMode::Spreadsheet,
            PageType::Sheet,
        );
        model.coordinate_system.unit = "synthetic".to_string();

        let pipeline_config = load_pipeline_config();
        let options = read_xlsx_options(pipeline_config.as_ref());
        if !options.enabled {
            model.warnings.push(warning(
                "XLSX_EXTRACTION_DISABLED",
                "XLSX extraction disabled by pipeline.office.enabled=false",
            ));
            return Ok(model);
        }

        let mut workbook = open_workbook_auto(input_path)
            .with_context(|| format!("XLSX_WORKBOOK_OPEN_FAILED: {}", input_path.display()))?;
        let package = OoxmlPackage::open(input_path).ok();

        model.pages.clear();
        let sheet_names = workbook.sheet_names().to_vec();
        if sheet_names.is_empty() {
            model.warnings.push(doc_warning(
                "XLSX_EMPTY_WORKBOOK",
                "Книга XLSX не содержит листов".to_string(),
            ));
            model.processing.status = ProcessingStatus::Partial;
            return Ok(model);
        }

        let mut global_order = 1u32;
        let mut has_formulas = false;
        let mut has_tables = false;
        let mut has_images = false;
        let document_id = model.document_id.clone();

        for (sheet_idx, name) in sheet_names.iter().enumerate() {
            if !options.extract_sheets {
                continue;
            }

            let range = match workbook.worksheet_range(name) {
                Ok(r) => r,
                Err(err) => {
                    model.warnings.push(doc_warning(
                        "XLSX_SHEET_READ_FAILED",
                        format!("Не удалось прочитать лист '{}': {}", name, err),
                    ));
                    continue;
                }
            };

            let mut rows = range
                .rows()
                .map(|row| row.iter().map(cell_to_string).collect::<Vec<_>>())
                .collect::<Vec<_>>();
            if options.used_range_only {
                rows = trim_empty_sheet_rows(rows);
            }

            let table_ranges = if options.detect_tables {
                detect_xlsx_table_ranges(&rows)
            } else if !rows.is_empty() {
                vec![crate::tables::TableRange {
                    start_row: 0,
                    end_row: rows.len().saturating_sub(1),
                    start_col: 0,
                    end_col: rows.iter().map(|r| r.len()).max().unwrap_or(1).saturating_sub(1),
                }]
            } else {
                vec![]
            };

            let mut elements = Vec::new();
            let mut local_order = 1u32;
            let mut y = 0f32;
            let used_range_a1 = used_range_to_a1(&rows);
            let mut page_text_lines = Vec::new();
            let mut page_markdown_lines = vec![format!("# {}", name)];

            for (range_idx, tr) in table_ranges.iter().enumerate() {
                let row_count = tr.end_row.saturating_sub(tr.start_row) + 1;
                if row_count == 0 {
                    continue;
                }

                let max_rows = options.max_rows_per_table_element.max(1);
                let mut segment_start = tr.start_row;
                while segment_start <= tr.end_row {
                    let segment_end = (segment_start + max_rows - 1).min(tr.end_row);
                    let rows_len = segment_end.saturating_sub(segment_start) + 1;
                    let cols_len = tr.end_col.saturating_sub(tr.start_col) + 1;
                    if rows_len == 0 || cols_len == 0 {
                        break;
                    }

                    let mut cells = Vec::new();
                    let mut formula_cells = 0usize;
                    for r in segment_start..=segment_end {
                        for c in tr.start_col..=tr.end_col {
                            let val = rows
                                .get(r)
                                .and_then(|row| row.get(c))
                                .cloned()
                                .unwrap_or_default();
                            let formula = detect_formula(&val, options.extract_formulas);
                            if formula.is_some() {
                                formula_cells += 1;
                                has_formulas = true;
                            }
                            cells.push(TableCell {
                                row: r - segment_start,
                                column: c - tr.start_col,
                                rowspan: 1,
                                colspan: 1,
                                bbox: None,
                                text: normalize_office_text(&val),
                                html: None,
                                markdown: None,
                                formula,
                                is_header: r == segment_start,
                                confidence: None,
                            });
                        }
                    }

                    let markdown = table_to_markdown(&cells, rows_len, cols_len);
                    let csv = table_to_csv(&cells, rows_len, cols_len);
                    let html = table_to_html(&cells, rows_len, cols_len);
                    let text = cells
                        .iter()
                        .map(|c| c.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" | ");

                    let linearized = linearize_cells(
                        &cells,
                        rows_len,
                        cols_len,
                        TableLinearizationOptions {
                            max_rows_per_chunk: options.max_rows_per_linearized_chunk.max(1),
                            language: "ru".to_string(),
                        },
                    );

                    let mut table = Element {
                        element_id: format!(
                            "sheet{}_table{}_{}",
                            sheet_idx + 1,
                            range_idx + 1,
                            segment_start + 1
                        ),
                        element_type: ElementType::Table,
                        tag: Some("sheet_range".to_string()),
                        role: Some("spreadsheet_range".to_string()),
                        reading_order: Some(local_order),
                        global_order: Some(global_order),
                        bbox: Some([
                            0.0,
                            y,
                            1000.0,
                            y + (rows_len as f32 * 18.0).max(20.0),
                        ]),
                        polygon: None,
                        content: json!({
                            "text": text,
                            "html": html,
                            "markdown": markdown,
                            "csv": csv,
                            "normalized_text": normalize_office_text(&text),
                            "raw": format!("{}!{}", name, used_range_a1),
                        }),
                        style: empty_style(),
                        provenance: json!({
                            "method": "native",
                            "tool": "xlsx_ooxml_parser",
                            "stage": "xlsx_extract_tables",
                            "source_ref": {
                                "kind": "sheet_range",
                                "value": format!("{}!{}", name, used_range_a1),
                            }
                        }),
                        confidence: default_confidence(),
                        warnings: vec![],
                        extra: HashMap::new(),
                    };
                    table.extra.insert("rows".to_string(), json!(rows_len));
                    table.extra.insert("columns".to_string(), json!(cols_len));
                    table.extra.insert(
                        "cells".to_string(),
                        serde_json::to_value(&cells).unwrap_or_else(|_| json!([])),
                    );
                    table.extra.insert(
                        "linearized_chunks".to_string(),
                        serde_json::to_value(&linearized).unwrap_or_else(|_| json!([])),
                    );
                    table.extra.insert(
                        "table_structure".to_string(),
                        serde_json::to_value(TableStructure {
                            has_header: true,
                            has_merged_cells: false,
                            orientation: "horizontal".to_string(),
                            extraction_method: "xlsx_used_range".to_string(),
                        })
                        .unwrap_or_else(|_| json!({})),
                    );
                    if formula_cells > 0 {
                        table.extra.insert("formula_cells".to_string(), json!(formula_cells));
                    }

                    page_text_lines.push(text);
                    page_markdown_lines.push(table.content["markdown"].as_str().unwrap_or_default().to_string());
                    has_tables = true;
                    y += (rows_len as f32 * 18.0).max(24.0) + 12.0;
                    local_order += 1;
                    global_order += 1;
                    elements.push(table);

                    if segment_end == tr.end_row {
                        break;
                    }
                    segment_start = segment_end + 1;
                }
            }

            let mut page = Page {
                page_number: (sheet_idx + 1) as u32,
                page_type: PageType::Sheet,
                width: None,
                height: None,
                dpi: None,
                rotation_degrees: 0.0,
                page_image_asset_id: None,
                page_profile: PageProfile {
                    content_mode: ContentMode::Sheet,
                    has_native_text: !rows.is_empty(),
                    has_ocr_required_regions: false,
                    has_tables: !elements.is_empty(),
                    has_images: false,
                    has_formulas,
                    has_handwriting: false,
                    language: Some("ru".to_string()),
                    language_info: crate::language::LanguageInfo::default(),
                    confidence: 0.95,
                },
                elements,
                text: page_text_lines.join("\n"),
                markdown: page_markdown_lines.join("\n\n"),
                html: String::new(),
                warnings: vec![],
                extra: {
                    let mut extra = HashMap::new();
                    extra.insert(
                        "sheet".to_string(),
                        json!({
                            "name": name,
                            "index": sheet_idx,
                            "used_range": used_range_a1,
                        }),
                    );
                    extra
                },
            };

            if options.extract_images {
                if let Some(pkg) = &package {
                    let image_count_before = model.assets.len();
                    let img_elements = extract_xlsx_media_assets(
                        pkg,
                        &document_id,
                        &mut model,
                        &mut page,
                        local_order,
                        global_order,
                    )?;
                    if !img_elements.is_empty() {
                        has_images = true;
                        page.page_profile.has_images = true;
                    }
                    if model.assets.len() > image_count_before {
                        page.page_profile.has_images = true;
                    }
                }
            }

            if options.extract_comments {
                model.warnings.push(doc_warning(
                    "XLSX_COMMENTS_SKIPPED",
                    "Комментарии XLSX пока пропущены: текущий backend не извлекает comments.xml"
                        .to_string(),
                ));
            }

            model.pages.push(page);
        }

        model.document_profile.has_tables = has_tables;
        model.document_profile.has_images = has_images;
        model.document_profile.has_formulas = has_formulas;
        model.document_profile.has_native_text = true;
        model.document_profile.document_type_guess = Some("spreadsheet".to_string());

        model
            .processing
            .stages
            .push(stage("xlsx_open_workbook", "calamine", 1));
        model
            .processing
            .stages
            .push(stage("xlsx_extract_sheets", "xlsx_ooxml_parser", 1));
        model
            .processing
            .stages
            .push(stage("xlsx_detect_ranges", "xlsx_ooxml_parser", 1));
        model
            .processing
            .stages
            .push(stage("xlsx_extract_tables", "xlsx_ooxml_parser", 1));
        model
            .processing
            .stages
            .push(stage("xlsx_extract_formulas", "xlsx_ooxml_parser", 1));
        model
            .processing
            .stages
            .push(stage("xlsx_extract_media", "xlsx_ooxml_parser", 1));
        model
            .processing
            .stages
            .push(stage("xlsx_chunking", "semantic_chunker", 0));

        update_stats(&mut model);
        model.processing.total_duration_ms = Some(model.processing.stages.len() as u64);

        Ok(model)
    }
}

fn extract_xlsx_media_assets(
    package: &OoxmlPackage,
    document_id: &str,
    model: &mut crate::model::DocumentModel,
    page: &mut Page,
    local_order_start: u32,
    global_order_start: u32,
) -> anyhow::Result<Vec<Element>> {
    let entries = package.list_entries();
    let media_entries = list_media_entries(&entries, "xl/media/");
    if media_entries.is_empty() {
        return Ok(vec![]);
    }

    let store: Box<dyn AssetStore + Send + Sync> = Box::new(LocalAssetStore::new(output_root_dir()));
    let mut elements = Vec::new();
    let mut local_order = local_order_start;
    let mut global_order = global_order_start;
    let mut seen_assets = HashSet::new();

    for (idx, entry) in media_entries.iter().enumerate() {
        let Some(bytes) = package.read_bytes(entry)? else {
            continue;
        };
        let file_name = entry.split('/').next_back().unwrap_or("xlsx_image.bin");

        let mut asset = store.write_asset(
            document_id,
            AssetType::EmbeddedImage,
            file_name,
            &bytes,
            content_type_for_media_path(entry),
        )?;
        asset.provenance = json!({
            "source": "xlsx_embedded_media",
            "tool": "xlsx_ooxml_parser",
            "stage": "xlsx_extract_media",
            "source_ref": {
                "kind": "zip_entry",
                "value": entry,
            }
        });

        if !seen_assets.contains(&asset.asset_id) {
            seen_assets.insert(asset.asset_id.clone());
            model.assets.push(asset.clone());
        }

        let mut extra = HashMap::new();
        extra.insert("asset_id".to_string(), json!(asset.asset_id));

        let mut element = Element {
            element_id: format!("sheet{}_image_{}", page.page_number, idx + 1),
            element_type: ElementType::Image,
            tag: Some("xdr:pic".to_string()),
            role: Some("embedded_image".to_string()),
            reading_order: Some(local_order),
            global_order: Some(global_order),
            bbox: Some([0.0, 0.0, 200.0, 120.0]),
            polygon: None,
            content: json!({
                "text": "",
                "markdown": "![XLSX image]",
                "html": null,
                "normalized_text": "",
                "raw": entry,
            }),
            style: empty_style(),
            provenance: json!({
                "method": "native",
                "tool": "xlsx_ooxml_parser",
                "stage": "xlsx_extract_media",
                "source_ref": {
                    "kind": "zip_entry",
                    "value": entry,
                }
            }),
            confidence: default_confidence(),
            warnings: vec![Diagnostic {
                code: "XLSX_IMAGE_POSITION_APPROXIMATE".to_string(),
                severity: "warning".to_string(),
                scope: "element".to_string(),
                page_number: Some(page.page_number),
                element_id: None,
                message: "Позиция изображения XLSX задана приближенно (synthetic bbox).".to_string(),
                recoverable: true,
                extra: HashMap::new(),
            }],
            extra,
        };
        element.extra.insert("language".to_string(), json!("ru"));

        page.elements.push(element.clone());
        elements.push(element);

        local_order += 1;
        global_order += 1;
    }

    Ok(elements)
}

fn detect_formula(value: &str, enabled: bool) -> Option<String> {
    if !enabled {
        return None;
    }
    let trimmed = value.trim();
    if trimmed.starts_with('=') && trimmed.len() > 1 {
        return Some(trimmed.to_string());
    }
    None
}

fn trim_empty_sheet_rows(mut rows: Vec<Vec<String>>) -> Vec<Vec<String>> {
    while rows
        .last()
        .map(|r| r.iter().all(|v| v.trim().is_empty()))
        .unwrap_or(false)
    {
        rows.pop();
    }
    rows
}

fn used_range_to_a1(rows: &[Vec<String>]) -> String {
    if rows.is_empty() {
        return "A1:A1".to_string();
    }
    let row_count = rows.len();
    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
    format!("A1:{}{}", column_to_a1(col_count - 1), row_count)
}

fn column_to_a1(mut idx: usize) -> String {
    let mut out = String::new();
    loop {
        let rem = idx % 26;
        out.insert(0, (b'A' + rem as u8) as char);
        if idx < 26 {
            break;
        }
        idx = idx / 26 - 1;
    }
    out
}

fn cell_to_string<T: ToString>(cell: &T) -> String {
    normalize_office_text(&cell.to_string())
}

fn read_xlsx_options(config: Option<&PipelineConfig>) -> XlsxOptions {
    let mut options = XlsxOptions::default();
    options.enabled = bool_opt(config, &["office", "enabled"], options.enabled);
    options.extract_sheets = bool_opt(config, &["office", "xlsx", "extract_sheets"], true);
    options.extract_cells = bool_opt(config, &["office", "xlsx", "extract_cells"], true);
    options.extract_formulas = bool_opt(config, &["office", "xlsx", "extract_formulas"], true);
    options.extract_comments = bool_opt(config, &["office", "xlsx", "extract_comments"], true);
    options.extract_images = bool_opt(config, &["office", "xlsx", "extract_images"], true);
    options.detect_tables = bool_opt(config, &["office", "xlsx", "detect_tables"], true);
    options.used_range_only = bool_opt(config, &["office", "xlsx", "used_range_only"], true);
    options.max_rows_per_table_element = u64_opt(
        config,
        &["office", "xlsx", "max_rows_per_table_element"],
        options.max_rows_per_table_element as u64,
    ) as usize;
    options.max_rows_per_linearized_chunk = u64_opt(
        config,
        &["office", "xlsx", "max_rows_per_linearized_chunk"],
        options.max_rows_per_linearized_chunk as u64,
    ) as usize;
    options
}

fn bool_opt(config: Option<&PipelineConfig>, path: &[&str], default: bool) -> bool {
    let Some(cfg) = config else {
        return default;
    };

    let mut cursor = json!(cfg.pipeline);
    for segment in path {
        let Some(next) = cursor.get(segment) else {
            return default;
        };
        cursor = next.clone();
    }
    cursor.as_bool().unwrap_or(default)
}

fn u64_opt(config: Option<&PipelineConfig>, path: &[&str], default: u64) -> u64 {
    let Some(cfg) = config else {
        return default;
    };

    let mut cursor = json!(cfg.pipeline);
    for segment in path {
        let Some(next) = cursor.get(segment) else {
            return default;
        };
        cursor = next.clone();
    }
    cursor.as_u64().unwrap_or(default)
}

fn load_pipeline_config() -> Option<PipelineConfig> {
    let path = std::path::Path::new("configs/pipeline.config.jsonc");
    crate::config::load_pipeline_config(path).ok()
}

fn doc_warning(code: &str, message: String) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity: "warning".to_string(),
        scope: "document".to_string(),
        page_number: None,
        element_id: None,
        message,
        recoverable: true,
        extra: HashMap::new(),
    }
}
