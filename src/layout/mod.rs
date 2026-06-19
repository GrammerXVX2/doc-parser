pub mod debug;
pub mod fixture_detector;
pub mod headers_footers;
pub mod heuristic_detector;
pub mod mock_detector;
pub mod reading_order;
pub mod traits;
pub mod types;

use serde_json::Value;

use crate::config::PipelineConfig;
use crate::runtime::pipeline_cli_overrides;

pub use fixture_detector::FixtureLayoutDetector;
pub use headers_footers::{
    HeaderFooterDetectionResult, apply_header_footer_marks, detect_repeated_headers_footers,
};
pub use heuristic_detector::HeuristicLayoutDetector;
pub use mock_detector::MockLayoutDetector;
pub use reading_order::{ReadingOrderOptions, assign_layout_aware_reading_order};
pub use traits::{LayoutDetectionInput, LayoutDetector};
pub use types::{
    LayoutRegion, LayoutRegionType, LayoutSource, layout_region_to_element_placeholder,
    should_create_placeholder,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutBackend {
    Heuristic,
    Mock,
    Fixture,
    Disabled,
}

#[derive(Debug, Clone)]
pub struct LayoutOptions {
    pub enabled: bool,
    pub backend: LayoutBackend,
    pub save_debug_artifacts: bool,
    pub detect_headers_footers: bool,
    pub detect_watermarks: bool,
    pub detect_columns: bool,
    pub detect_figures: bool,
    pub detect_tables: bool,
    pub detect_formulas: bool,
    pub reading_order: ReadingOrderOptions,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: LayoutBackend::Heuristic,
            save_debug_artifacts: false,
            detect_headers_footers: true,
            detect_watermarks: true,
            detect_columns: true,
            detect_figures: true,
            detect_tables: true,
            detect_formulas: true,
            reading_order: ReadingOrderOptions::default(),
        }
    }
}

pub fn resolve_layout_options(config: Option<&PipelineConfig>) -> LayoutOptions {
    let mut out = LayoutOptions::default();
    let layout = config
        .and_then(|cfg| cfg.pipeline.layout.as_object())
        .cloned()
        .unwrap_or_default();

    out.enabled = read_bool_map(&layout, "enabled", out.enabled);
    out.save_debug_artifacts = read_bool_map(&layout, "save_debug_artifacts", out.save_debug_artifacts);
    out.detect_headers_footers = read_bool_map(&layout, "detect_headers_footers", out.detect_headers_footers);
    out.detect_watermarks = read_bool_map(&layout, "detect_watermarks", out.detect_watermarks);
    out.detect_columns = read_bool_map(&layout, "detect_columns", out.detect_columns);
    out.detect_figures = read_bool_map(&layout, "detect_figures", out.detect_figures);
    out.detect_tables = read_bool_map(&layout, "detect_tables", out.detect_tables);
    out.detect_formulas = read_bool_map(&layout, "detect_formulas", out.detect_formulas);
    out.backend = parse_backend(
        layout
            .get("backend")
            .and_then(Value::as_str)
            .unwrap_or("heuristic"),
    );

    if let Some(ro) = layout.get("reading_order").and_then(Value::as_object) {
        out.reading_order.strategy = ro
            .get("strategy")
            .and_then(Value::as_str)
            .unwrap_or(&out.reading_order.strategy)
            .to_string();
        out.reading_order.multi_column = ro
            .get("multi_column")
            .and_then(Value::as_bool)
            .unwrap_or(out.reading_order.multi_column);
        out.reading_order.header_footer_handling = ro
            .get("header_footer_handling")
            .and_then(Value::as_str)
            .unwrap_or(&out.reading_order.header_footer_handling)
            .to_string();
        out.reading_order.y_tolerance = ro
            .get("y_tolerance")
            .and_then(Value::as_f64)
            .map(|v| v as f32)
            .unwrap_or(out.reading_order.y_tolerance);
        out.reading_order.column_gap_threshold = ro
            .get("column_gap_threshold")
            .and_then(Value::as_f64)
            .map(|v| v as f32)
            .unwrap_or(out.reading_order.column_gap_threshold);
    }

    if let Some(overrides) = pipeline_cli_overrides() {
        if let Some(backend) = overrides.layout_backend.as_deref() {
            out.backend = parse_backend(backend);
            out.enabled = out.backend != LayoutBackend::Disabled;
        }
        if let Some(reading_order) = overrides.reading_order.as_deref() {
            out.reading_order.strategy = reading_order.to_string();
        }
        if let Some(debug_layout) = overrides.debug_layout {
            out.save_debug_artifacts = debug_layout;
        }
        if let Some(exclude) = overrides.exclude_headers_footers_from_chunks {
            if exclude {
                out.reading_order.header_footer_handling = "exclude_from_chunks".to_string();
            }
        }
    }

    out
}

pub fn build_layout_detector(options: &LayoutOptions) -> Box<dyn LayoutDetector> {
    match options.backend {
        LayoutBackend::Heuristic => Box::new(HeuristicLayoutDetector),
        LayoutBackend::Mock => Box::new(MockLayoutDetector),
        LayoutBackend::Fixture => Box::new(FixtureLayoutDetector),
        LayoutBackend::Disabled => Box::new(MockLayoutDetector),
    }
}

fn read_bool_map(map: &serde_json::Map<String, Value>, key: &str, default: bool) -> bool {
    map.get(key).and_then(Value::as_bool).unwrap_or(default)
}

fn parse_backend(value: &str) -> LayoutBackend {
    match value.to_ascii_lowercase().as_str() {
        "mock" => LayoutBackend::Mock,
        "fixture" => LayoutBackend::Fixture,
        "disabled" => LayoutBackend::Disabled,
        _ => LayoutBackend::Heuristic,
    }
}
