# Implementation Plan

## Phase 1: Core data model

Create Rust structs for:

```text
DocumentModel
SourceInfo
DocumentProfile
CoordinateSystem
Page
Element
Asset
Chunk
ProcessingTrace
PipelineError
```

Requirements:

- derive Serialize / Deserialize;
- stable field names;
- optional fields for format-specific data;
- strict enum types where possible;
- JSON output must match `schemas/document_model.schema.jsonc`.

## Phase 2: File classifier

Implement:

```text
classify_file(input_path) -> FileClassification
```

Must detect:

- extension;
- MIME by extension;
- MIME by magic bytes;
- file size;
- sha256;
- likely document type;
- encrypted/protected if possible.

## Phase 3: Format router

Implement router:

```text
route_document(classification) -> Extractor
```

Extractors:

```text
PdfExtractor
DocxExtractor
HtmlExtractor
MarkdownExtractor
RtfExtractor
ImageExtractor
PptxExtractor
TxtExtractor
XlsxExtractor
```

Each extractor must output intermediate canonical representation.

## Phase 4: Native extractors

### HTML

- parse DOM;
- extract headings;
- paragraphs;
- lists;
- blockquotes;
- pre/code;
- tables;
- images;
- formulas if present.

### Markdown

- parse Markdown AST;
- extract headings/lists/code/tables/images;
- optionally convert to HTML.

### TXT

- detect encoding;
- normalize line endings;
- create synthetic pages;
- detect simple structure.

### XLSX

- parse workbook;
- each sheet as page;
- extract cells/tables/formulas/comments.

### DOCX/PPTX

- parse OOXML ZIP;
- extract XML;
- extract media;
- extract paragraphs/tables/images/slides.

### PDF

- native text extraction;
- image extraction;
- page metadata;
- optional render.

## Phase 5: Rendering layer

Implement abstraction:

```text
Renderer
  render_page(document, page_number, options) -> PageRenderAsset
```

Backends:

- PDF renderer;
- HTML browser renderer;
- Office-to-PDF renderer via LibreOffice;
- image passthrough renderer.

## Phase 6: OCR layer

Implement OCR as optional stage.

Interfaces:

```text
OcrDetector
  detect_text_regions(image) -> Vec<TextRegion>

OcrRecognizer
  recognize(crops) -> Vec<RecognizedText>

OcrPipeline
  run(page_image) -> Vec<Element>
```

Support dynamic batching:

```text
max_batch_size
max_wait_ms
preferred_batch_sizes
```

## Phase 7: Table recognition

For native tables:

- parse directly.

For scanned tables:

- detect table region;
- recognize structure;
- OCR cells;
- output table element.

## Phase 8: Formula recognition

For native formulas:

- extract LaTeX/MathML.

For scanned formulas:

- crop formula region;
- recognize to LaTeX.

## Phase 9: Merge and dedup

Implement:

```text
merge_native_and_ocr(native_elements, ocr_elements) -> Vec<Element>
```

Dedup by:

- bbox IoU;
- text similarity;
- source priority;
- confidence.

Priority:

```text
native text > high-confidence OCR > low-confidence OCR
native table > table recognition > plain OCR table text
native formula > formula OCR
```

## Phase 10: Reading order

Implement:

```text
assign_reading_order(elements) -> Vec<Element>
```

MVP:

- group by layout regions;
- sort top-to-bottom;
- sort left-to-right.

Advanced:

- layout graph;
- multi-column detection;
- table/caption binding;
- header/footer detection.

## Phase 11: Chunking

Implement semantic chunker.

Rules:

- start new chunk on major heading;
- keep table as separate chunk or table-aware chunk;
- keep code block intact;
- include captions with images;
- preserve formulas;
- limit token size.

## Phase 12: Output writer

Write:

```text
model.json
markdown.md
plain_text.txt
assets/
```

Validate JSON against schema.

## Phase 13: Observability

Add metrics:

```text
documents_total
documents_failed_total
pages_total
pages_ocr_total
stage_duration_ms
queue_depth
gpu_batch_size
gpu_inference_ms
errors_by_code
```

Add structured logs with:

```text
document_id
job_id
page_number
stage
duration_ms
status
```

## Phase 14: Testing

Test cases:

```text
digital.pdf
scanned.pdf
hybrid.pdf
simple.docx
docx_with_images.docx
markdown_with_tables.md
html_article.html
rtf_file.rtf
scan.jpg
presentation.pptx
spreadsheet.xlsx
plain.txt
corrupted.pdf
password_protected.pdf
```

Assertions:

- model.json is valid;
- no duplicate text;
- pages count correct;
- elements have IDs;
- OCR elements have confidence;
- assets exist;
- chunks are generated.