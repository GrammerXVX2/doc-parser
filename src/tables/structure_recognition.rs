use async_trait::async_trait;

use crate::converters::traits::{ConversionError, ConversionStageRecord, ExtractionContext, Result};
use crate::model::Element;
use crate::tables::placeholder::create_scanned_table_placeholder;
use crate::tables::scanned_detector::TableRegion;

#[derive(Debug, Clone)]
pub struct TableStructureInput {
    pub document_id: String,
    pub table_region: TableRegion,
}

#[async_trait]
pub trait TableStructureRecognizer: Send + Sync {
    async fn recognize_structure(
        &self,
        input: TableStructureInput,
        context: &mut ExtractionContext,
    ) -> Result<Element>;
}

#[derive(Debug, Default, Clone)]
pub struct MockTableStructureRecognizer;

#[async_trait]
impl TableStructureRecognizer for MockTableStructureRecognizer {
    async fn recognize_structure(
        &self,
        input: TableStructureInput,
        context: &mut ExtractionContext,
    ) -> Result<Element> {
        context.push_stage(
            ConversionStageRecord::ok("table_structure_recognition", "mock_table_structure_recognizer")
                .with_meta("region_id", input.table_region.region_id.clone()),
        );

        let mut element = create_scanned_table_placeholder(
            input.table_region.page_number,
            &input.table_region.region_id,
            input.table_region.bbox,
            input.table_region.confidence,
            "mock_table_structure_recognizer",
        );
        element
            .extra
            .insert("rows".to_string(), serde_json::json!(2));
        element
            .extra
            .insert("columns".to_string(), serde_json::json!(2));
        Ok(element)
    }
}

#[derive(Debug, Default, Clone)]
pub struct FixtureTableStructureRecognizer;

#[async_trait]
impl TableStructureRecognizer for FixtureTableStructureRecognizer {
    async fn recognize_structure(
        &self,
        input: TableStructureInput,
        context: &mut ExtractionContext,
    ) -> Result<Element> {
        context.push_stage(
            ConversionStageRecord::ok("table_structure_recognition", "fixture_table_structure_recognizer")
                .with_meta("region_id", input.table_region.region_id.clone()),
        );

        Ok(create_scanned_table_placeholder(
            input.table_region.page_number,
            &input.table_region.region_id,
            input.table_region.bbox,
            input.table_region.confidence,
            "fixture_table_structure_recognizer",
        ))
    }
}

#[derive(Debug, Default, Clone)]
pub struct DisabledTableStructureRecognizer;

#[async_trait]
impl TableStructureRecognizer for DisabledTableStructureRecognizer {
    async fn recognize_structure(
        &self,
        input: TableStructureInput,
        context: &mut ExtractionContext,
    ) -> Result<Element> {
        context.push_stage(
            ConversionStageRecord::warning("table_structure_recognition", "disabled_table_structure_recognizer")
                .with_meta("region_id", input.table_region.region_id.clone()),
        );

        Err(ConversionError::new(
            "TABLE_STRUCTURE_RECOGNITION_FAILED",
            "Распознавание структуры таблицы отключено для текущего backend.",
        ))
    }
}
