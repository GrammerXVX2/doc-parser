pub mod block_reconstruction;
pub mod classifier;
pub mod layout_hints;
pub mod native;
pub mod renderer;
pub mod spans;
pub mod types;

pub use block_reconstruction::{
	merge_lines_into_blocks, merge_spans_into_lines, pdf_blocks_to_elements,
};
pub use classifier::{classify_pdf_by_text, split_pdf_text_to_pages};
pub use native::{extract_native_pages, native_text_to_elements, text_to_synthetic_spans};
pub use spans::{PdfTextBlock, PdfTextLine, PdfTextReconstructionOptions, PdfTextSpan};
pub use types::{PdfClassification, PdfPageClassification, PdfPageContentMode, PdfPageNativeText};
