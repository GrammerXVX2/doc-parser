use std::collections::{HashMap, HashSet};
use std::path::Path;

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
use crate::office::comments::extract_comments_texts;
use crate::office::docx::{DocxBlockKind, extract_ordered_blocks};
use crate::office::formulas::{contains_omml, extract_first_omml};
use crate::office::media::{
    content_type_for_media_path, list_media_entries, normalize_relationship_target,
};
use crate::office::numbering::{NumberingInfo, is_ordered_list, parse_numbering};
use crate::office::ooxml::OoxmlPackage;
use crate::office::relationships::{Relationship, parse_relationships};
use crate::office::styles::{DocxStyles, parse_docx_styles};
use crate::router::Extractor;
use crate::runtime::output_root_dir;
use crate::tables::{
    TableCell, TableLinearizationOptions, TableStructure, linearize_cells, table_to_csv,
    table_to_html, table_to_markdown,
};
use crate::utils::office_text::{heading_level, is_heading_style, normalize_office_text};
use crate::utils::xml::{extract_xml_texts, strip_xml_tags};

#[derive(Default)]
pub struct DocxExtractor;

#[derive(Debug, Clone)]
struct DocxOptions {
    enabled: bool,
    extract_paragraphs: bool,
    extract_headings: bool,
    extract_lists: bool,
    extract_tables: bool,
    extract_images: bool,
    extract_footnotes: bool,
    extract_endnotes: bool,
    extract_comments: bool,
    extract_headers: bool,
    extract_footers: bool,
    extract_formulas: bool,
    extract_embedded_images: bool,
    synthetic_page_line_limit: usize,
}

impl Default for DocxOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            extract_paragraphs: true,
            extract_headings: true,
            extract_lists: true,
            extract_tables: true,
            extract_images: true,
            extract_footnotes: true,
            extract_endnotes: true,
            extract_comments: true,
            extract_headers: true,
            extract_footers: true,
            extract_formulas: true,
            extract_embedded_images: true,
            synthetic_page_line_limit: 80,
        }
    }
}

#[derive(Debug, Clone)]
struct CandidateElement {
    element: Element,
    estimated_lines: usize,
}

#[derive(Debug, Clone)]
struct PendingList {
    list_type: String,
    items: Vec<String>,
}

impl Extractor for DocxExtractor {
    fn name(&self) -> &'static str {
        "docx_extractor"
    }

    fn extract(
        &self,
        input_path: &Path,
        classification: &FileClassification,
    ) -> anyhow::Result<crate::model::DocumentModel> {
        let mut model = base_document_model(
            classification,
            DocumentFormat::Docx,
            ContentMode::Digital,
            PageType::DocumentPage,
        );
        model.coordinate_system.unit = "synthetic".to_string();

        let pipeline_config = load_pipeline_config();
        let options = read_docx_options(pipeline_config.as_ref());
        if !options.enabled {
            model.warnings.push(warning(
                "DOCX_EXTRACTION_DISABLED",
                "DOCX extraction disabled by pipeline.office.enabled=false",
            ));
            return Ok(model);
        }

        let package = match OoxmlPackage::open(input_path) {
            Ok(pkg) => pkg,
            Err(err) => {
                model.errors.push(doc_error(
                    "DOCX_INVALID_PACKAGE",
                    format!("Не удалось открыть DOCX пакет: {err}"),
                ));
                model.processing.status = ProcessingStatus::Partial;
                return Ok(model);
            }
        };
        model
            .processing
            .stages
            .push(stage("docx_open_package", "docx_ooxml_parser", 1));

        let Some(document_xml) = package.read_text("word/document.xml")? else {
            model.errors.push(doc_error(
                "DOCX_DOCUMENT_XML_MISSING",
                "В DOCX отсутствует обязательный файл word/document.xml".to_string(),
            ));
            model.processing.status = ProcessingStatus::Partial;
            return Ok(model);
        };

        let relationships = parse_docx_relationships(&package, &mut model);
        model
            .processing
            .stages
            .push(stage("docx_parse_relationships", "docx_ooxml_parser", 1));

        let styles = parse_docx_styles_safe(&package, &mut model);
        model
            .processing
            .stages
            .push(stage("docx_parse_styles", "docx_ooxml_parser", 1));

        let numbering = parse_docx_numbering_safe(&package, &mut model);
        model
            .processing
            .stages
            .push(stage("docx_parse_numbering", "docx_ooxml_parser", 1));

        let store: Box<dyn AssetStore + Send + Sync> = Box::new(LocalAssetStore::new(output_root_dir()));
        let mut candidates = Vec::new();
        let mut pending_list: Option<PendingList> = None;
        let mut paragraph_count = 0usize;
        let mut list_count = 0usize;
        let mut table_count = 0usize;
        let mut image_count = 0usize;
        let mut notes_count = 0usize;
        let document_id = model.document_id.clone();
        let mut cached_assets: HashMap<String, String> = HashMap::new();
        let mut registered_assets: HashSet<String> = HashSet::new();

        let blocks = extract_ordered_blocks(&document_xml)?;
        for block in blocks {
            match block.kind {
                DocxBlockKind::Table => {
                    flush_pending_list(&mut pending_list, &mut candidates, &mut list_count);
                    if !options.extract_tables {
                        continue;
                    }
                    if let Some(table_element) = make_table_element(&block.xml, table_count + 1) {
                        candidates.push(table_element);
                        table_count += 1;
                    }
                }
                DocxBlockKind::Paragraph => {
                    let paragraph_text = extract_xml_texts(&block.xml, "w:t").join("");
                    let paragraph_text = normalize_office_text(&paragraph_text);
                    let style_id = paragraph_style_id(&block.xml).unwrap_or_default();
                    let list_num_id = paragraph_num_id(&block.xml);
                    let embedded_rids = paragraph_embedded_rids(&block.xml);

                    if options.extract_lists
                        && list_num_id.is_some()
                        && !paragraph_text.trim().is_empty()
                    {
                        let list_type = if is_ordered_list(
                            list_num_id.as_deref().unwrap_or_default(),
                            &numbering,
                        ) {
                            "ol"
                        } else {
                            "ul"
                        }
                        .to_string();

                        match &mut pending_list {
                            Some(list) if list.list_type == list_type => {
                                list.items.push(paragraph_text.clone());
                            }
                            Some(_) => {
                                flush_pending_list(&mut pending_list, &mut candidates, &mut list_count);
                                pending_list = Some(PendingList {
                                    list_type,
                                    items: vec![paragraph_text.clone()],
                                });
                            }
                            None => {
                                pending_list = Some(PendingList {
                                    list_type,
                                    items: vec![paragraph_text.clone()],
                                });
                            }
                        }
                    } else {
                        flush_pending_list(&mut pending_list, &mut candidates, &mut list_count);
                        if !paragraph_text.trim().is_empty() && options.extract_paragraphs {
                            if let Some(elem) = make_paragraph_element(
                                &paragraph_text,
                                &style_id,
                                &styles,
                                options.extract_headings,
                                paragraph_count + 1,
                            ) {
                                candidates.push(elem);
                                paragraph_count += 1;
                            }
                        }
                    }

                    if options.extract_formulas && contains_omml(&block.xml) {
                        if let Some(omml) = extract_first_omml(&block.xml) {
                            let formula_text = strip_xml_tags(&omml);
                            candidates.push(CandidateElement {
                                element: Element {
                                    element_id: String::new(),
                                    element_type: ElementType::Formula,
                                    tag: Some("m:oMath".to_string()),
                                    role: Some("equation".to_string()),
                                    reading_order: None,
                                    global_order: None,
                                    bbox: None,
                                    polygon: None,
                                    content: json!({
                                        "text": formula_text,
                                        "markdown": formula_text,
                                        "html": null,
                                        "normalized_text": formula_text,
                                        "raw": omml,
                                        "latex": null,
                                    }),
                                    style: empty_style(),
                                    provenance: json!({
                                        "method": "native",
                                        "tool": "docx_ooxml_parser",
                                        "stage": "docx_formula_extraction",
                                        "source_ref": {
                                            "kind": "xml",
                                            "value": "word/document.xml"
                                        }
                                    }),
                                    confidence: default_confidence(),
                                    warnings: vec![],
                                    extra: {
                                        let mut extra = HashMap::new();
                                        extra.insert("format".to_string(), json!("omml"));
                                        extra
                                    },
                                },
                                estimated_lines: 2,
                            });
                        }
                    }

                    if options.extract_images && options.extract_embedded_images {
                        for rid in embedded_rids {
                            match materialize_docx_image(
                                &rid,
                                &relationships,
                                &package,
                                &*store,
                                &document_id,
                                &mut cached_assets,
                                &mut model,
                                &mut registered_assets,
                            ) {
                                Ok(Some(asset_id)) => {
                                    candidates.push(CandidateElement {
                                        element: make_image_element(image_count + 1, &asset_id),
                                        estimated_lines: 6,
                                    });
                                    image_count += 1;
                                }
                                Ok(None) => {}
                                Err(err) => {
                                    model.warnings.push(doc_warning(
                                        "DOCX_IMAGE_EXTRACTION_FAILED",
                                        format!(
                                            "Не удалось извлечь изображение DOCX ({}): {}",
                                            rid, err
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        flush_pending_list(&mut pending_list, &mut candidates, &mut list_count);

        if options.extract_footnotes {
            notes_count += extract_note_elements(
                &package,
                "word/footnotes.xml",
                ElementType::Footnote,
                "footnote",
                "docx_extract_notes_comments",
                &mut candidates,
            )?;
        }
        if options.extract_endnotes {
            notes_count += extract_note_elements(
                &package,
                "word/endnotes.xml",
                ElementType::Footnote,
                "endnote",
                "docx_extract_notes_comments",
                &mut candidates,
            )?;
        }
        if options.extract_comments {
            notes_count += extract_comment_elements(
                &package,
                "word/comments.xml",
                "docx_extract_notes_comments",
                &mut candidates,
            )?;
        }
        if options.extract_headers {
            for entry in list_media_entries(&package.list_entries(), "word/header") {
                if entry.ends_with(".xml") {
                    notes_count += extract_note_elements(
                        &package,
                        &entry,
                        ElementType::Header,
                        "header",
                        "docx_extract_notes_comments",
                        &mut candidates,
                    )?;
                }
            }
        }
        if options.extract_footers {
            for entry in list_media_entries(&package.list_entries(), "word/footer") {
                if entry.ends_with(".xml") {
                    notes_count += extract_note_elements(
                        &package,
                        &entry,
                        ElementType::Footer,
                        "footer",
                        "docx_extract_notes_comments",
                        &mut candidates,
                    )?;
                }
            }
        }

        model
            .processing
            .stages
            .push(stage("docx_extract_paragraphs", "docx_ooxml_parser", 1));
        model
            .processing
            .stages
            .push(stage("docx_extract_lists", "docx_ooxml_parser", 1));
        model
            .processing
            .stages
            .push(stage("docx_extract_tables", "docx_ooxml_parser", 1));
        model
            .processing
            .stages
            .push(stage("docx_extract_media", "docx_ooxml_parser", 1));
        model.processing.stages.push(stage(
            "docx_extract_notes_comments",
            "docx_ooxml_parser",
            1,
        ));
        model
            .processing
            .stages
            .push(stage("docx_chunking", "semantic_chunker", 0));

        model.pages = paginate_candidates(
            candidates,
            options.synthetic_page_line_limit.max(20),
            notes_count,
        );
        if model.pages.is_empty() {
            model.pages.push(empty_docx_page(1));
        }

        model.document_profile.has_images = !model.assets.is_empty();
        model.document_profile.has_tables = model
            .pages
            .iter()
            .flat_map(|p| p.elements.iter())
            .any(|e| matches!(e.element_type, ElementType::Table));
        model.document_profile.has_formulas = model
            .pages
            .iter()
            .flat_map(|p| p.elements.iter())
            .any(|e| matches!(e.element_type, ElementType::Formula));
        model.document_profile.document_type_guess = Some("word_document".to_string());

        update_stats(&mut model);
        model.processing.total_duration_ms = Some(model.processing.stages.len() as u64);

        Ok(model)
    }
}

fn parse_docx_relationships(package: &OoxmlPackage, model: &mut crate::model::DocumentModel) -> Vec<Relationship> {
    let Ok(rels_text) = package.read_text("word/_rels/document.xml.rels") else {
        model.warnings.push(doc_warning(
            "DOCX_MEDIA_RELATIONSHIP_MISSING",
            "Не найден файл связей word/_rels/document.xml.rels; извлечение медиа может быть неполным."
                .to_string(),
        ));
        return vec![];
    };

    let Some(xml) = rels_text else {
        model.warnings.push(doc_warning(
            "DOCX_MEDIA_RELATIONSHIP_MISSING",
            "Не найден файл связей word/_rels/document.xml.rels; извлечение медиа может быть неполным."
                .to_string(),
        ));
        return vec![];
    };

    match parse_relationships(&xml) {
        Ok(v) => v,
        Err(err) => {
            model.warnings.push(doc_warning(
                "DOCX_MEDIA_RELATIONSHIP_MISSING",
                format!("Не удалось разобрать relationships DOCX: {err}"),
            ));
            vec![]
        }
    }
}

fn parse_docx_styles_safe(package: &OoxmlPackage, model: &mut crate::model::DocumentModel) -> DocxStyles {
    match package.read_text("word/styles.xml") {
        Ok(Some(xml)) => parse_docx_styles(&xml),
        Ok(None) => DocxStyles::default(),
        Err(err) => {
            model.warnings.push(doc_warning(
                "DOCX_STYLE_PARSE_FAILED",
                format!("Не удалось разобрать стили DOCX: {err}"),
            ));
            DocxStyles::default()
        }
    }
}

fn parse_docx_numbering_safe(
    package: &OoxmlPackage,
    model: &mut crate::model::DocumentModel,
) -> NumberingInfo {
    match package.read_text("word/numbering.xml") {
        Ok(Some(xml)) => parse_numbering(&xml),
        Ok(None) => NumberingInfo::default(),
        Err(err) => {
            model.warnings.push(doc_warning(
                "DOCX_NUMBERING_PARSE_FAILED",
                format!(
                    "Не удалось разобрать настройки нумерации DOCX. Списки будут восстановлены приближенно: {err}"
                ),
            ));
            NumberingInfo::default()
        }
    }
}

fn make_paragraph_element(
    text: &str,
    style_id: &str,
    styles: &DocxStyles,
    allow_headings: bool,
    index: usize,
) -> Option<CandidateElement> {
    if text.trim().is_empty() {
        return None;
    }

    let heading_by_style = is_heading_style(style_id)
        || styles.heading_styles.contains(style_id)
        || styles.heading_styles.contains(&style_id.to_lowercase());

    let is_heading = allow_headings
        && (heading_by_style
            || (text.len() < 100 && text.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)));

    let monospace = styles.monospace_styles.contains(style_id);
    let (element_type, tag, role, markdown) = if monospace {
        (
            ElementType::Code,
            Some("code".to_string()),
            Some("code_block".to_string()),
            format!("```\n{text}\n```"),
        )
    } else if is_heading {
        let lvl = heading_level(style_id);
        (
            ElementType::Heading,
            Some(format!("h{lvl}")),
            Some("section_title".to_string()),
            format!("{} {text}", "#".repeat(lvl)),
        )
    } else {
        (
            ElementType::Paragraph,
            Some("p".to_string()),
            Some("paragraph".to_string()),
            text.to_string(),
        )
    };

    Some(CandidateElement {
        element: Element {
            element_id: format!("docx_paragraph_{index}"),
            element_type,
            tag,
            role,
            reading_order: None,
            global_order: None,
            bbox: None,
            polygon: None,
            content: json!({
                "text": text,
                "markdown": markdown,
                "html": null,
                "normalized_text": normalize_office_text(text),
                "raw": text,
            }),
            style: empty_style(),
            provenance: json!({
                "method": "native",
                "tool": "docx_ooxml_parser",
                "stage": "docx_paragraph_extraction",
                "source_ref": {
                    "kind": "xml",
                    "value": "word/document.xml"
                }
            }),
            confidence: default_confidence(),
            warnings: vec![],
            extra: HashMap::new(),
        },
        estimated_lines: estimate_lines(text),
    })
}

fn make_table_element(table_xml: &str, index: usize) -> Option<CandidateElement> {
    let Ok(row_re) = regex::Regex::new(r"(?s)<w:tr\b[^>]*>(.*?)</w:tr>") else {
        return None;
    };
    let Ok(cell_re) = regex::Regex::new(r"(?s)<w:tc\b[^>]*>(.*?)</w:tc>") else {
        return None;
    };

    let mut cells = Vec::new();
    let mut rows = 0usize;
    let mut cols = 0usize;
    let mut has_merged = false;

    for (r, row_cap) in row_re.captures_iter(table_xml).enumerate() {
        let Some(row_inner) = row_cap.get(1).map(|m| m.as_str()) else {
            continue;
        };
        rows = rows.max(r + 1);
        let mut c = 0usize;
        for cell_cap in cell_re.captures_iter(row_inner) {
            let Some(cell_inner) = cell_cap.get(1).map(|m| m.as_str()) else {
                continue;
            };
            let text = normalize_office_text(&extract_xml_texts(cell_inner, "w:t").join(""));
            let colspan = scoped_attr_value(cell_inner, "<w:gridSpan", "w:val=")
                .or_else(|| scoped_attr_value(cell_inner, "<w:gridSpan", "val="))
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(1);
            let rowspan = 1usize;
            if colspan > 1 || cell_inner.contains("<w:vMerge") {
                has_merged = true;
            }

            cells.push(TableCell {
                row: r,
                column: c,
                rowspan,
                colspan,
                bbox: None,
                text,
                html: None,
                markdown: None,
                formula: None,
                is_header: r == 0,
                confidence: None,
            });
            c += 1;
        }
        cols = cols.max(c);
    }

    if rows == 0 || cols == 0 || cells.is_empty() {
        return None;
    }

    let markdown = table_to_markdown(&cells, rows, cols);
    let csv = table_to_csv(&cells, rows, cols);
    let html = table_to_html(&cells, rows, cols);
    let text = cells
        .iter()
        .map(|c| c.text.as_str())
        .collect::<Vec<_>>()
        .join(" | ");
    let linearized = linearize_cells(
        &cells,
        rows,
        cols,
        TableLinearizationOptions {
            max_rows_per_chunk: 20,
            language: "ru".to_string(),
        },
    );

    let mut element = Element {
        element_id: format!("docx_table_{index}"),
        element_type: ElementType::Table,
        tag: Some("w:tbl".to_string()),
        role: Some("data_table".to_string()),
        reading_order: None,
        global_order: None,
        bbox: None,
        polygon: None,
        content: json!({
            "text": text,
            "markdown": markdown,
            "html": html,
            "csv": csv,
            "normalized_text": normalize_office_text(&text),
            "raw": table_xml,
        }),
        style: empty_style(),
        provenance: json!({
            "method": "native",
            "tool": "docx_ooxml_parser",
            "stage": "docx_table_extraction",
            "source_ref": {
                "kind": "xml",
                "value": "word/document.xml"
            }
        }),
        confidence: default_confidence(),
        warnings: vec![],
        extra: HashMap::new(),
    };

    element.extra.insert("rows".to_string(), json!(rows));
    element.extra.insert("columns".to_string(), json!(cols));
    element.extra.insert(
        "cells".to_string(),
        serde_json::to_value(&cells).unwrap_or_else(|_| json!([])),
    );
    element.extra.insert(
        "linearized_chunks".to_string(),
        serde_json::to_value(&linearized).unwrap_or_else(|_| json!([])),
    );
    element.extra.insert(
        "table_structure".to_string(),
        serde_json::to_value(TableStructure {
            has_header: true,
            has_merged_cells: has_merged,
            orientation: "horizontal".to_string(),
            extraction_method: "docx_native".to_string(),
        })
        .unwrap_or_else(|_| json!({})),
    );

    Some(CandidateElement {
        element,
        estimated_lines: rows + 1,
    })
}

fn make_image_element(index: usize, asset_id: &str) -> Element {
    let mut extra = HashMap::new();
    extra.insert("asset_id".to_string(), json!(asset_id));

    Element {
        element_id: format!("docx_image_{index}"),
        element_type: ElementType::Image,
        tag: Some("w:drawing".to_string()),
        role: Some("embedded_image".to_string()),
        reading_order: None,
        global_order: None,
        bbox: None,
        polygon: None,
        content: json!({
            "text": "",
            "markdown": "![DOCX image]",
            "html": null,
            "normalized_text": "",
            "raw": null,
            "alt": null,
            "description": {"short": null, "method": null}
        }),
        style: empty_style(),
        provenance: json!({
            "method": "native",
            "tool": "docx_ooxml_parser",
            "stage": "docx_media_extraction",
        }),
        confidence: default_confidence(),
        warnings: vec![],
        extra,
    }
}

fn materialize_docx_image(
    rid: &str,
    relationships: &[Relationship],
    package: &OoxmlPackage,
    store: &dyn AssetStore,
    document_id: &str,
    cached_assets: &mut HashMap<String, String>,
    model: &mut crate::model::DocumentModel,
    registered_assets: &mut HashSet<String>,
) -> anyhow::Result<Option<String>> {
    if let Some(asset_id) = cached_assets.get(rid) {
        return Ok(Some(asset_id.clone()));
    }

    let Some(rel) = relationships.iter().find(|r| r.id == rid) else {
        return Ok(None);
    };
    if rel
        .target_mode
        .as_deref()
        .map(|m| m.eq_ignore_ascii_case("External"))
        .unwrap_or(false)
    {
        return Ok(None);
    }

    let target = normalize_relationship_target("word", &rel.target);
    let Some(bytes) = package.read_bytes(&target)? else {
        return Ok(None);
    };

    let file_name = target
        .split('/')
        .next_back()
        .unwrap_or("docx_image.bin");
    let mut asset = store.write_asset(
        document_id,
        AssetType::EmbeddedImage,
        file_name,
        &bytes,
        content_type_for_media_path(&target),
    )?;
    asset.provenance = json!({
        "source": "docx_embedded_image",
        "tool": "docx_ooxml_parser",
        "stage": "docx_media_extraction",
        "source_ref": {
            "kind": "relationship",
            "value": rid,
        }
    });

    let asset_id = asset.asset_id.clone();
    if !registered_assets.contains(&asset_id) {
        model.assets.push(asset);
        registered_assets.insert(asset_id.clone());
    }
    cached_assets.insert(rid.to_string(), asset_id.clone());

    Ok(Some(asset_id))
}

fn extract_note_elements(
    package: &OoxmlPackage,
    entry: &str,
    element_type: ElementType,
    role: &str,
    stage_name: &str,
    out: &mut Vec<CandidateElement>,
) -> anyhow::Result<usize> {
    let Some(xml) = package.read_text(entry)? else {
        return Ok(0);
    };

    let texts = extract_xml_texts(&xml, "w:t")
        .into_iter()
        .map(|t| normalize_office_text(&t))
        .filter(|t| !t.trim().is_empty())
        .collect::<Vec<_>>();

    for text in &texts {
        out.push(CandidateElement {
            element: Element {
                element_id: String::new(),
                element_type: element_type.clone(),
                tag: Some("w:p".to_string()),
                role: Some(role.to_string()),
                reading_order: None,
                global_order: None,
                bbox: None,
                polygon: None,
                content: json!({
                    "text": text,
                    "markdown": text,
                    "html": null,
                    "normalized_text": text,
                    "raw": entry,
                }),
                style: empty_style(),
                provenance: json!({
                    "method": "native",
                    "tool": "docx_ooxml_parser",
                    "stage": stage_name,
                    "source_ref": {"kind": "xml", "value": entry}
                }),
                confidence: default_confidence(),
                warnings: vec![],
                extra: HashMap::new(),
            },
            estimated_lines: estimate_lines(text),
        });
    }

    Ok(texts.len())
}

fn extract_comment_elements(
    package: &OoxmlPackage,
    entry: &str,
    stage_name: &str,
    out: &mut Vec<CandidateElement>,
) -> anyhow::Result<usize> {
    let Some(xml) = package.read_text(entry)? else {
        return Ok(0);
    };

    let comments = extract_comments_texts(&xml)
        .into_iter()
        .map(|t| normalize_office_text(&t))
        .filter(|t| !t.trim().is_empty())
        .collect::<Vec<_>>();

    for text in &comments {
        out.push(CandidateElement {
            element: Element {
                element_id: String::new(),
                element_type: ElementType::Footnote,
                tag: Some("w:comment".to_string()),
                role: Some("comment".to_string()),
                reading_order: None,
                global_order: None,
                bbox: None,
                polygon: None,
                content: json!({
                    "text": text,
                    "markdown": text,
                    "html": null,
                    "normalized_text": text,
                    "raw": entry,
                }),
                style: empty_style(),
                provenance: json!({
                    "method": "native",
                    "tool": "docx_ooxml_parser",
                    "stage": stage_name,
                    "source_ref": {"kind": "xml", "value": entry}
                }),
                confidence: default_confidence(),
                warnings: vec![],
                extra: HashMap::new(),
            },
            estimated_lines: estimate_lines(text),
        });
    }

    Ok(comments.len())
}

fn flush_pending_list(
    pending: &mut Option<PendingList>,
    out: &mut Vec<CandidateElement>,
    list_count: &mut usize,
) {
    let Some(list) = pending.take() else {
        return;
    };

    if list.items.is_empty() {
        return;
    }

    let markdown = if list.list_type == "ol" {
        list.items
            .iter()
            .enumerate()
            .map(|(i, t)| format!("{}. {}", i + 1, t))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        list.items
            .iter()
            .map(|t| format!("- {}", t))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let text = list.items.join("\n");
    out.push(CandidateElement {
        element: Element {
            element_id: format!("docx_list_{}", *list_count + 1),
            element_type: ElementType::List,
            tag: Some("w:numPr".to_string()),
            role: Some(if list.list_type == "ol" {
                "ordered_list"
            } else {
                "unordered_list"
            }
            .to_string()),
            reading_order: None,
            global_order: None,
            bbox: None,
            polygon: None,
            content: json!({
                "text": text,
                "markdown": markdown,
                "html": null,
                "normalized_text": normalize_office_text(&text),
                "raw": text,
                "items": list.items,
            }),
            style: empty_style(),
            provenance: json!({
                "method": "native",
                "tool": "docx_ooxml_parser",
                "stage": "docx_list_extraction",
                "source_ref": {"kind": "xml", "value": "word/document.xml"}
            }),
            confidence: default_confidence(),
            warnings: vec![],
            extra: {
                let mut extra = HashMap::new();
                extra.insert("list_type".to_string(), json!(list.list_type));
                extra
            },
        },
        estimated_lines: list.items.len().max(1),
    });
    *list_count += 1;
}

fn paginate_candidates(candidates: Vec<CandidateElement>, line_limit: usize, _notes_count: usize) -> Vec<Page> {
    if candidates.is_empty() {
        return vec![empty_docx_page(1)];
    }

    let mut pages = Vec::new();
    let mut current = Vec::new();
    let mut current_lines = 0usize;
    let mut page_number = 1u32;
    let mut global_order = 1u32;

    for mut candidate in candidates {
        let needed = candidate.estimated_lines.max(1);
        if current_lines + needed > line_limit && !current.is_empty() {
            pages.push(build_docx_page(page_number, std::mem::take(&mut current)));
            page_number += 1;
            current_lines = 0;
        }

        let local_order = current.len() as u32 + 1;
        let y0 = current_lines as f32 * 16.0;
        let h = needed as f32 * 16.0;
        candidate.element.reading_order = Some(local_order);
        candidate.element.global_order = Some(global_order);
        candidate.element.element_id = format!("p{}_e{}", page_number, local_order);
        candidate.element.bbox = Some([0.0, y0, 1000.0, y0 + h]);
        current.push(candidate.element);

        current_lines += needed;
        global_order += 1;
    }

    if !current.is_empty() {
        pages.push(build_docx_page(page_number, current));
    }

    pages
}

fn build_docx_page(page_number: u32, elements: Vec<Element>) -> Page {
    let text = elements
        .iter()
        .filter_map(|e| e.content.get("text").and_then(|v| v.as_str()))
        .filter(|t| !t.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let markdown = elements
        .iter()
        .filter_map(|e| e.content.get("markdown").and_then(|v| v.as_str()))
        .filter(|t| !t.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    let has_tables = elements.iter().any(|e| matches!(e.element_type, ElementType::Table));
    let has_images = elements.iter().any(|e| matches!(e.element_type, ElementType::Image));
    let has_formulas = elements
        .iter()
        .any(|e| matches!(e.element_type, ElementType::Formula));

    Page {
        page_number,
        page_type: PageType::DocumentPage,
        width: Some(1000.0),
        height: Some(1400.0),
        dpi: None,
        rotation_degrees: 0.0,
        page_image_asset_id: None,
        page_profile: PageProfile {
            content_mode: ContentMode::Digital,
            has_native_text: !text.trim().is_empty(),
            has_ocr_required_regions: false,
            has_tables,
            has_images,
            has_formulas,
            has_handwriting: false,
            language: Some("ru".to_string()),
            language_info: crate::language::LanguageInfo::default(),
            confidence: 0.92,
        },
        elements,
        text,
        markdown,
        html: String::new(),
        warnings: vec![],
        extra: HashMap::new(),
    }
}

fn empty_docx_page(page_number: u32) -> Page {
    Page {
        page_number,
        page_type: PageType::DocumentPage,
        width: Some(1000.0),
        height: Some(1400.0),
        dpi: None,
        rotation_degrees: 0.0,
        page_image_asset_id: None,
        page_profile: PageProfile {
            content_mode: ContentMode::Digital,
            has_native_text: false,
            has_ocr_required_regions: false,
            has_tables: false,
            has_images: false,
            has_formulas: false,
            has_handwriting: false,
            language: Some("ru".to_string()),
            language_info: crate::language::LanguageInfo::default(),
            confidence: 0.8,
        },
        elements: vec![],
        text: String::new(),
        markdown: String::new(),
        html: String::new(),
        warnings: vec![],
        extra: HashMap::new(),
    }
}

fn paragraph_style_id(xml: &str) -> Option<String> {
    scoped_attr_value(xml, "<w:pStyle", "w:val=")
        .or_else(|| scoped_attr_value(xml, "<w:pStyle", "val="))
}

fn paragraph_num_id(xml: &str) -> Option<String> {
    scoped_attr_value(xml, "<w:numId", "w:val=")
        .or_else(|| scoped_attr_value(xml, "<w:numId", "val="))
}

fn paragraph_embedded_rids(xml: &str) -> Vec<String> {
    let Ok(re) = regex::Regex::new(r#"r:embed=\"([^\"]+)\""#) else {
        return vec![];
    };
    re.captures_iter(xml)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect::<Vec<_>>()
}

fn scoped_attr_value(scope: &str, tag: &str, marker: &str) -> Option<String> {
    let idx = scope.find(tag)?;
    let scoped = &scope[idx..];
    attr_value(scoped, marker)
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

fn estimate_lines(text: &str) -> usize {
    let hard_lines = text.lines().count().max(1);
    let soft = text.chars().count() / 80 + 1;
    hard_lines.max(soft)
}

fn read_docx_options(config: Option<&PipelineConfig>) -> DocxOptions {
    let mut options = DocxOptions::default();
    options.enabled = bool_opt(config, &["office", "enabled"], options.enabled);

    options.extract_paragraphs = bool_opt(config, &["office", "docx", "extract_paragraphs"], true);
    options.extract_headings = bool_opt(config, &["office", "docx", "extract_headings"], true);
    options.extract_lists = bool_opt(config, &["office", "docx", "extract_lists"], true);
    options.extract_tables = bool_opt(config, &["office", "docx", "extract_tables"], true);
    options.extract_images = bool_opt(config, &["office", "docx", "extract_images"], true);
    options.extract_footnotes = bool_opt(config, &["office", "docx", "extract_footnotes"], true);
    options.extract_endnotes = bool_opt(config, &["office", "docx", "extract_endnotes"], true);
    options.extract_comments = bool_opt(config, &["office", "docx", "extract_comments"], true);
    options.extract_headers = bool_opt(config, &["office", "docx", "extract_headers"], true);
    options.extract_footers = bool_opt(config, &["office", "docx", "extract_footers"], true);
    options.extract_formulas = bool_opt(config, &["office", "docx", "extract_formulas"], true);
    options.extract_embedded_images = bool_opt(
        config,
        &["office", "embedded_images", "extract"],
        true,
    );
    options.synthetic_page_line_limit = u64_opt(
        config,
        &["office", "docx", "synthetic_page_line_limit"],
        options.synthetic_page_line_limit as u64,
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

fn doc_error(code: &str, message: String) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity: "error".to_string(),
        scope: "document".to_string(),
        page_number: None,
        element_id: None,
        message,
        recoverable: true,
        extra: HashMap::new(),
    }
}
