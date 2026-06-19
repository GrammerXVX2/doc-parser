use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::extractors::{default_confidence, empty_style};
use crate::model::{Diagnostic, Element, ElementType};
use crate::utils::geometry::{BBox, Point};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayoutRegionType {
    Text,
    Title,
    List,
    Table,
    Figure,
    Formula,
    Header,
    Footer,
    Footnote,
    Caption,
    Watermark,
    Sidebar,
    Code,
    Unknown,
}

impl LayoutRegionType {
    pub fn from_str(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "text" => Self::Text,
            "title" => Self::Title,
            "list" => Self::List,
            "table" => Self::Table,
            "figure" => Self::Figure,
            "formula" => Self::Formula,
            "header" => Self::Header,
            "footer" => Self::Footer,
            "footnote" => Self::Footnote,
            "caption" => Self::Caption,
            "watermark" => Self::Watermark,
            "sidebar" => Self::Sidebar,
            "code" => Self::Code,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Title => "title",
            Self::List => "list",
            Self::Table => "table",
            Self::Figure => "figure",
            Self::Formula => "formula",
            Self::Header => "header",
            Self::Footer => "footer",
            Self::Footnote => "footnote",
            Self::Caption => "caption",
            Self::Watermark => "watermark",
            Self::Sidebar => "sidebar",
            Self::Code => "code",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayoutSource {
    Heuristic,
    Mock,
    Fixture,
    Model,
    NativePdf,
}

impl LayoutSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Heuristic => "heuristic",
            Self::Mock => "mock",
            Self::Fixture => "fixture",
            Self::Model => "model",
            Self::NativePdf => "native_pdf",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutRegion {
    pub region_id: String,
    pub page_number: usize,
    pub region_type: LayoutRegionType,
    pub bbox: BBox,
    pub polygon: Option<Vec<Point>>,
    pub confidence: f32,
    pub reading_order: Option<usize>,
    pub source: LayoutSource,
}

pub fn should_create_placeholder(region_type: &LayoutRegionType) -> bool {
    matches!(
        region_type,
        LayoutRegionType::Table
            | LayoutRegionType::Formula
            | LayoutRegionType::Figure
            | LayoutRegionType::Watermark
            | LayoutRegionType::Unknown
    )
}

pub fn layout_region_to_element_placeholder(region: &LayoutRegion) -> Element {
    let (element_type, role, text) = match region.region_type {
        LayoutRegionType::Table => (
            ElementType::Table,
            "layout_table_placeholder",
            "[Обнаружена область таблицы макета]",
        ),
        LayoutRegionType::Formula => (
            ElementType::Formula,
            "layout_formula_placeholder",
            "[Обнаружена область формулы макета]",
        ),
        LayoutRegionType::Figure => (
            ElementType::Image,
            "layout_figure_placeholder",
            "[Обнаружена область изображения/фигуры]",
        ),
        LayoutRegionType::Watermark => (
            ElementType::Watermark,
            "layout_watermark",
            "[Обнаружен возможный watermark]",
        ),
        _ => (
            ElementType::Unknown,
            "layout_unknown_region",
            "[Обнаружена область неизвестного типа]",
        ),
    };

    let mut extra = HashMap::new();
    extra.insert("layout_region_id".to_string(), json!(region.region_id));
    extra.insert("layout_source".to_string(), json!(region.source.as_str()));
    extra.insert("layout_region_type".to_string(), json!(region.region_type.as_str()));

    Element {
        element_id: format!("p{}_layout_{}", region.page_number, region.region_id),
        element_type,
        tag: Some("layout_region".to_string()),
        role: Some(role.to_string()),
        reading_order: region.reading_order.map(|v| v as u32),
        global_order: None,
        bbox: Some(region.bbox.to_array()),
        polygon: region
            .polygon
            .as_ref()
            .map(|poly| poly.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>()),
        content: json!({
            "text": text,
            "markdown": text,
            "html": null,
            "normalized_text": text,
            "raw": null,
        }),
        style: empty_style(),
        provenance: json!({
            "method": "inferred",
            "tool": format!("{}_layout_detector", region.source.as_str()),
            "stage": "layout_detection",
            "source_ref": {
                "kind": "layout_region",
                "value": region.region_id,
            }
        }),
        confidence: {
            let mut conf = default_confidence();
            conf["overall"] = json!(region.confidence);
            conf
        },
        warnings: vec![Diagnostic {
            code: "LAYOUT_REGION_PLACEHOLDER".to_string(),
            severity: "warning".to_string(),
            scope: "element".to_string(),
            page_number: Some(region.page_number as u32),
            element_id: None,
            message: "Создан placeholder элемент на основе layout-региона.".to_string(),
            recoverable: true,
            extra: HashMap::new(),
        }],
        extra,
    }
}
