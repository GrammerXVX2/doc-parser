use std::path::Path;

use super::mock_renderer::MockRenderer;
use super::traits::{PageRenderer, RenderOptions, RenderedPage};

#[derive(Debug, Default)]
pub struct PdfRenderer {
    mock: MockRenderer,
}

impl PageRenderer for PdfRenderer {
    fn render_page(
        &self,
        input_path: &Path,
        page_number: usize,
        options: RenderOptions,
    ) -> anyhow::Result<RenderedPage> {
        // Stage 2 fallback implementation delegates to mock renderer when real backend is absent.
        self.mock.render_page(input_path, page_number, options)
    }
}
