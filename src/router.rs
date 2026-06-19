use std::path::Path;

use thiserror::Error;

use crate::classifier::{DetectedFormat, FileClassification};
use crate::extractors::doc::DocExtractor;
use crate::extractors::docx::DocxExtractor;
use crate::extractors::html::HtmlExtractor;
use crate::extractors::image::ImageExtractor;
use crate::extractors::markdown::MarkdownExtractor;
use crate::extractors::pdf::PdfExtractor;
use crate::extractors::pptx::PptxExtractor;
use crate::extractors::rtf::RtfExtractor;
use crate::extractors::txt::TxtExtractor;
use crate::extractors::xlsx::XlsxExtractor;

pub trait Extractor {
    fn name(&self) -> &'static str;
    fn extract(
        &self,
        input_path: &Path,
        classification: &FileClassification,
    ) -> anyhow::Result<crate::model::DocumentModel>;
}

#[derive(Debug, Error)]
pub enum RouterError {
    #[error("extractor is not implemented for format: {0}")]
    NotImplemented(String),
}

#[derive(Default)]
pub struct FormatRouter {
    pdf: PdfExtractor,
    docx: DocxExtractor,
    doc: DocExtractor,
    html: HtmlExtractor,
    markdown: MarkdownExtractor,
    rtf: RtfExtractor,
    image: ImageExtractor,
    pptx: PptxExtractor,
    txt: TxtExtractor,
    xlsx: XlsxExtractor,
}

impl FormatRouter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn route(&self, classification: &FileClassification) -> anyhow::Result<&dyn Extractor> {
        let extractor: &dyn Extractor = match classification.likely_format {
            DetectedFormat::Pdf => &self.pdf,
            DetectedFormat::Docx => &self.docx,
            DetectedFormat::Doc => &self.doc,
            DetectedFormat::Html => &self.html,
            DetectedFormat::Md => &self.markdown,
            DetectedFormat::Rtf => &self.rtf,
            DetectedFormat::Image => &self.image,
            DetectedFormat::Pptx => &self.pptx,
            DetectedFormat::Txt => &self.txt,
            DetectedFormat::Xlsx => &self.xlsx,
            _ => {
                return Err(RouterError::NotImplemented(format!(
                    "{:?}",
                    classification.likely_format
                ))
                .into());
            }
        };

        Ok(extractor)
    }
}
