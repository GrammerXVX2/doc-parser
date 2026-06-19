# Data Model

## 1. Document

Root object:

```text
DocumentModel
  schema_version
  document_id
  job_id
  source
  document_profile
  stats
  coordinate_system
  assets
  pages
  chunks
  errors
  warnings
  processing
```

## 2. Source

Contains original file info.

```json
{
  "uri": "s3://bucket/file.pdf",
  "filename": "file.pdf",
  "extension": "pdf",
  "mime_type": "application/pdf",
  "size_bytes": 123456,
  "hashes": {
    "sha256": "..."
  }
}
```

## 3. Document profile

Describes document after classification.

```json
{
  "format": "pdf",
  "content_mode": "hybrid",
  "languages": ["ru", "en"],
  "has_native_text": true,
  "has_images": true,
  "has_tables": true,
  "has_formulas": false,
  "has_ocr_required_regions": true,
  "document_type_guess": "contract",
  "confidence": 0.91
}
```

## 4. Coordinate system

All bboxes use page coordinate system.

```json
{
  "origin": "top_left",
  "unit": "px",
  "dpi": 144,
  "normalized_to_page": true
}
```

## 5. Page

Page can represent:

- PDF page;
- DOCX rendered page;
- HTML simulated page;
- PPTX slide;
- XLSX sheet;
- image;
- TXT synthetic page.

```json
{
  "page_number": 1,
  "page_type": "document_page",
  "width": 1190,
  "height": 1684,
  "dpi": 144,
  "rotation_degrees": 0,
  "page_image_asset_id": "asset_page_1_render",
  "page_profile": {},
  "elements": [],
  "text": "",
  "markdown": "",
  "html": "",
  "warnings": []
}
```

## 6. Element

Common fields for all elements:

```json
{
  "element_id": "p1_e1",
  "type": "text",
  "tag": "p",
  "role": "paragraph",
  "reading_order": 1,
  "global_order": 1,
  "bbox": [0, 0, 100, 100],
  "polygon": null,
  "content": {},
  "style": {},
  "provenance": {},
  "confidence": {},
  "warnings": []
}
```

## 7. Element types

Supported types:

```text
text
text_ocr
heading
paragraph
blockquote
list
list_item
code
image
page_image
table
formula
caption
header
footer
footnote
watermark
chart
shape
unknown
```

## 8. Content object

Text-like content:

```json
{
  "text": "Plain text",
  "html": "<p>Plain text</p>",
  "markdown": "Plain text",
  "normalized_text": "plain text",
  "raw": "<p class=\"x\">Plain text</p>"
}
```

## 9. Provenance

Every element must explain where it came from.

```json
{
  "method": "native",
  "tool": "html_dom_parser",
  "stage": "html_extraction",
  "source_ref": {
    "kind": "css_selector",
    "value": "body > p:nth-of-type(1)"
  }
}
```

Allowed methods:

```text
native
ocr
converted
rendered
inferred
manual
```

## 10. Confidence

```json
{
  "overall": 0.98,
  "text": 0.99,
  "layout": 0.95,
  "structure": null,
  "language": 0.90
}
```

## 11. Assets

Asset object:

```json
{
  "asset_id": "asset_img_1",
  "type": "image",
  "path": "assets/images/image_1.jpg",
  "mime_type": "image/jpeg",
  "page_number": 1,
  "width": 1024,
  "height": 768,
  "dpi": null,
  "sha256": "...",
  "provenance": {
    "source": "embedded",
    "tool": "docx_media_extractor",
    "stage": "asset_extraction"
  }
}
```

Asset types:

```text
page_render
embedded_image
ocr_crop
table_html
table_csv
formula_image
debug
```

## 12. Chunks

Chunks are created for RAG and LLM usage.

```json
{
  "chunk_id": "chunk_1",
  "type": "section",
  "title": "Introduction",
  "page_start": 1,
  "page_end": 2,
  "element_ids": ["p1_e1", "p1_e2"],
  "text": "...",
  "markdown": "...",
  "token_estimate": 800,
  "metadata": {
    "language": "en",
    "contains_table": false,
    "contains_image": false,
    "contains_ocr": false,
    "section_path": ["Introduction"]
  }
}
```

## 13. Errors and warnings

Use structured errors.

```json
{
  "code": "PDF_RENDER_TIMEOUT",
  "severity": "error",
  "scope": "page",
  "page_number": 10,
  "message": "PDF render timed out",
  "recoverable": true
}
```

## 14. Processing trace

Each stage should write duration and status.

```json
{
  "pipeline_version": "1.0.0",
  "status": "ok",
  "stages": [
    {
      "name": "detect_file_type",
      "status": "ok",
      "tool": "infer",
      "duration_ms": 2
    }
  ],
  "total_duration_ms": 1200
}
```