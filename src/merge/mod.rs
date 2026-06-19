pub mod dedup;
pub mod geometry;
pub mod reading_order;
pub mod text_similarity;

pub use dedup::{DedupOptions, MergeOutcome, merge_native_and_ocr, merge_native_and_ocr_with_outcome};
pub use reading_order::ReadingOrderEngine;
pub use text_similarity::text_similarity;
