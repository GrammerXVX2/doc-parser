pub mod csv;
pub mod detect_txt;
pub mod html;
pub mod linearize;
pub mod markdown;
pub mod model;
pub mod placeholder;
pub mod scanned_detector;
pub mod structure_recognition;
pub mod xlsx_ranges;

pub use csv::table_to_csv;
pub use detect_txt::{detect_pipe_table, detect_tsv_table};
pub use html::table_to_html;
pub use linearize::{linearize_cells, linearize_table};
pub use markdown::table_to_markdown;
pub use model::{TableCell, TableLinearizationOptions, TableLinearizedChunk, TableStructure};
pub use placeholder::create_scanned_table_placeholder;
pub use scanned_detector::{
	DisabledScannedTableDetector, FixtureScannedTableDetector, MockScannedTableDetector,
	ScannedTableDetector, TableDetectionInput, TableRegion,
};
pub use structure_recognition::{
	DisabledTableStructureRecognizer, FixtureTableStructureRecognizer,
	MockTableStructureRecognizer, TableStructureInput, TableStructureRecognizer,
};
pub use xlsx_ranges::{TableRange, detect_xlsx_table_ranges};
