# Architecture

## 1. Общая архитектура

Система строится как асинхронный producer-consumer pipeline.

```text
┌─────────────────────────────┐
│ 1. Ingest                    │
│ - upload                     │
│ - local path                 │
│ - S3/object storage          │
└──────────────┬──────────────┘
               ↓
┌─────────────────────────────┐
│ 2. File Classifier           │
│ - extension                  │
│ - MIME                       │
│ - magic bytes                │
│ - encryption/password check  │
└──────────────┬──────────────┘
               ↓
┌─────────────────────────────┐
│ 3. Format Router             │
│ - PDF                        │
│ - DOCX/DOC                   │
│ - HTML/MD/RTF/TXT            │
│ - IMG                        │
│ - PPTX                       │
│ - XLSX                       │
└──────────────┬──────────────┘
               ↓
┌─────────────────────────────┐
│ 4. Native Extractors         │
│ - text                       │
│ - tables                     │
│ - images                     │
│ - formulas                   │
│ - styles                     │
│ - metadata                   │
└──────────────┬──────────────┘
               ↓
┌─────────────────────────────┐
│ 5. Renderers                 │
│ - PDF → page image           │
│ - DOCX/PPTX/XLSX → PDF/PNG   │
│ - HTML/MD → browser render   │
└──────────────┬──────────────┘
               ↓
┌─────────────────────────────┐
│ 6. Layout Analyzer           │
│ - text regions               │
│ - tables                     │
│ - figures                    │
│ - formulas                   │
│ - headers/footers            │
└──────────────┬──────────────┘
               ↓
┌─────────────────────────────┐
│ 7. OCR Workers               │
│ - detection                  │
│ - recognition                │
│ - table recognition          │
│ - formula recognition        │
└──────────────┬──────────────┘
               ↓
┌─────────────────────────────┐
│ 8. Merger / Dedup            │
│ - native + OCR merge         │
│ - duplicate detection        │
│ - confidence arbitration     │
└──────────────┬──────────────┘
               ↓
┌─────────────────────────────┐
│ 9. Reading Order             │
│ - columns                    │
│ - blocks                     │
│ - captions                   │
│ - footnotes                  │
└──────────────┬──────────────┘
               ↓
┌─────────────────────────────┐
│ 10. Chunker                  │
│ - sections                   │
│ - paragraphs                 │
│ - tables                     │
│ - code blocks                │
│ - image descriptions         │
└──────────────┬──────────────┘
               ↓
┌─────────────────────────────┐
│ 11. Output Writer            │
│ - model.json                 │
│ - markdown.md                │
│ - plain_text.txt             │
│ - assets                     │
└─────────────────────────────┘
```

## 2. Основные сущности

### DocumentJob

Входная задача обработки документа.

```text
DocumentJob
  document_id
  source_uri
  mime_type
  extension
  options
  priority
  timeout
```

### PageJob

Одна страница, слайд, лист или synthetic page.

```text
PageJob
  document_id
  page_number
  page_type
  page_image
  native_elements
  requires_ocr
```

### Element

Любой логический объект на странице:

- text;
- text_ocr;
- heading;
- paragraph;
- list;
- image;
- table;
- formula;
- code;
- blockquote;
- caption;
- header;
- footer;
- footnote;
- watermark;
- unknown.

### Asset

Внешний файл, связанный с документом:

- render страницы;
- embedded image;
- OCR crop;
- table HTML/CSV;
- formula image;
- debug artifact.

## 3. Async pipeline

Рекомендуемый runtime:

- `tokio` для async I/O;
- `rayon` для CPU-bound задач;
- отдельный GPU worker pool для inference;
- bounded queues для backpressure.

Пример очередей:

```text
ingest_queue
  ↓
classification_queue
  ↓
native_extraction_queue
  ↓
render_queue
  ↓
layout_queue
  ↓
ocr_detection_queue
  ↓
ocr_recognition_queue
  ↓
merge_queue
  ↓
output_queue
```

Все очереди должны быть bounded.

Нельзя использовать unbounded channels в production, потому что при всплеске документов система может съесть всю RAM.

## 4. Режимы работы

### Bulk mode

Для массовой обработки архива.

```text
max_batch_size: high
max_wait_ms: high
latency_priority: low
throughput_priority: high
```

### Online mode

Для API с низкой задержкой.

```text
max_batch_size: lower
max_wait_ms: 5-20
latency_priority: high
throughput_priority: medium
```

## 5. GPU OCR архитектура

OCR лучше разделить минимум на два этапа:

```text
Page image
  ↓
Text detection model
  ↓
Text boxes / polygons
  ↓
Crop builder
  ↓
Recognition model
  ↓
Text lines
```

Для таблиц и формул возможны отдельные модели:

```text
Table region
  ↓
Table structure recognition
  ↓
Cells
  ↓
OCR per cell or native text mapping
```

```text
Formula region
  ↓
Formula recognition
  ↓
LaTeX / MathML
```

## 6. Native + OCR merge

Гибридные документы требуют аккуратного объединения.

Алгоритм:

```text
for each OCR element:
  find native elements with overlapping bbox
  calculate IoU
  calculate text similarity
  if IoU > threshold and similarity > threshold:
    mark OCR as duplicate
  else:
    keep OCR element
```

## 7. Reading order

Reading order не должен равняться простому sort by y/x.

Нужно учитывать:

- multi-column layout;
- tables;
- captions;
- footnotes;
- sidebars;
- slides;
- floating text boxes;
- headers/footers.

Минимальный fallback:

```text
sort by block group
sort by y
sort by x
```

Лучший вариант:

```text
layout graph
  nodes = elements
  edges = visual/logical adjacency
  topological order = reading order
```

## 8. Output strategy

Для каждого документа:

```text
<document_id>/
  model.json
  markdown.md
  plain_text.txt
  assets/
    renders/
    images/
    tables/
    crops/
    formulas/
    debug/
```