# Technologies

## 1. Основной язык

Рекомендуемый основной язык:

```text
Rust
```

Причины:

- высокая производительность;
- memory safety;
- удобный async runtime;
- хорошая интеграция с системными библиотеками;
- удобно строить pipeline и backpressure;
- хорош для production-сервиса.

## 2. Rust dependencies

### Async runtime

```toml
tokio = { version = "1", features = ["full"] }
```

Используется для:

- async file I/O;
- HTTP API;
- очередей;
- таймаутов;
- orchestration.

### CPU parallelism

```toml
rayon = "1"
```

Используется для:

- image preprocessing;
- PDF page preprocessing;
- batch transforms;
- normalization;
- chunk preparation.

### Serialization

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_with = "3"
```

Используется для:

- `model.json`;
- config files;
- internal task payloads.

### Errors

```toml
thiserror = "1"
anyhow = "1"
```

`thiserror` — для typed errors внутри crate.  
`anyhow` — для верхнеуровневых application errors.

### Logging and tracing

```toml
tracing = "0.1"
tracing-subscriber = "0.3"
```

Используется для:

- structured logs;
- span per document;
- span per page;
- span per pipeline stage.

### IDs and time

```toml
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

## 3. File detection

```toml
infer = "0.16"
mime_guess = "2"
```

Используется для:

- magic bytes;
- MIME detection;
- fallback по extension.

## 4. Images

```toml
image = "0.25"
fast_image_resize = "5"
kamadak-exif = "0.5"
```

Используется для:

- decode;
- resize;
- orientation;
- normalization;
- crop extraction.

## 5. PDF

Варианты:

```toml
lopdf = "0.34"
pdfium-render = "0.8"
```

Внешние инструменты:

```text
PDFium
Poppler
MuPDF
Apache PDFBox
```

Рекомендация:

- для text/object extraction использовать native PDF extractor;
- для rendering использовать PDFium/Poppler/MuPDF;
- для сложных PDF иметь fallback.

## 6. HTML

```toml
scraper = "0.20"
html5ever = "0.27"
ammonia = "4"
```

Используется для:

- DOM parsing;
- text extraction;
- sanitization;
- extracting headings/lists/tables/images/code.

Для точных bbox HTML нужен browser render:

```text
Chromium
Playwright
Chrome DevTools Protocol
```

## 7. Markdown

```toml
pulldown-cmark = "0.12"
comrak = "0.29"
```

Используется для:

- Markdown AST;
- headings;
- lists;
- code fences;
- tables;
- links;
- images.

Fallback:

```text
Pandoc
```

## 8. Office formats

### DOCX / PPTX

```toml
zip = "2"
quick-xml = "0.36"
```

DOCX/PPTX — это OOXML packages.  
Можно извлекать:

- XML documents;
- relationships;
- embedded media;
- styles;
- tables;
- slides.

### XLSX

```toml
calamine = "0.26"
```

Используется для:

- workbook parsing;
- sheet extraction;
- cells;
- formulas;
- ranges.

### DOC / legacy formats

Для старых `.doc`, `.rtf`, сложных офисных файлов лучше использовать external converters:

```text
LibreOffice headless
Apache Tika
Pandoc
```

## 9. OCR / ML

### ONNX Runtime

```toml
ort = { version = "2", features = ["cuda", "tensorrt"] }
ndarray = "0.16"
half = "2"
```

Используется для:

- detection model;
- recognition model;
- table structure model;
- formula model.

### TensorRT

Использовать на production optimization stage.

Цели:

- FP16;
- INT8 при наличии calibration;
- static/dynamic shape profiles;
- engine cache;
- lower latency;
- higher throughput.

### Модели

Для классического OCR:

```text
PP-OCR style:
  detection model
  recognition model
  classification/orientation model
```

Для layout:

```text
Layout detection model
Document layout transformer
YOLO-like layout model
```

Для таблиц:

```text
Table structure recognition
Cell detection
Table OCR
```

Для формул:

```text
Formula OCR to LaTeX
```

Для сложных случаев:

```text
VLM slow path
```

## 10. Storage

```toml
object_store = { version = "0.11", features = ["aws", "gcp", "azure", "http"] }
```

Поддержка:

- local FS;
- S3;
- GCS;
- Azure Blob;
- HTTP.

## 11. Database

Варианты:

```text
PostgreSQL
SQLite for local mode
Object storage for assets
Vector DB for chunks
```

Для PostgreSQL:

```toml
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "json", "uuid", "chrono"] }
```

## 12. Observability

```toml
metrics = "0.24"
metrics-exporter-prometheus = "0.16"
opentelemetry = "0.25"
```

Метрики:

- documents_total;
- pages_total;
- pages_ocr_total;
- extraction_ms;
- rendering_ms;
- ocr_detection_ms;
- ocr_recognition_ms;
- table_recognition_ms;
- queue_depth;
- gpu_batch_size;
- gpu_latency_ms;
- errors_by_format.