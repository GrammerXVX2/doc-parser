# Parsing Rules

## 1. Общие правила

### Rule 1: Native first

Если документ содержит native text / native tables / native images, сначала извлекать их.

OCR использовать только если:

- страница image-only;
- регион не имеет native text;
- embedded image содержит текст;
- text layer повреждён;
- text layer слишком плохой;
- есть подозрение на скан;
- нужен OCR для таблицы/формулы/скриншота.

### Rule 2: Не дублировать текст

Если OCR нашёл текст поверх native text, нужно выполнить deduplication.

Критерии:

```text
IoU bbox > 0.5
AND text similarity > 0.8
```

Тогда OCR-элемент помечается как duplicate или удаляется из основного reading order.

### Rule 3: Всегда хранить provenance

Каждый элемент должен иметь:

```json
"provenance": {
  "method": "native | ocr | converted | inferred | rendered",
  "tool": "string",
  "stage": "string",
  "source_ref": {}
}
```

### Rule 4: Всегда хранить confidence

Для native extraction confidence может быть высоким.

Для OCR/ML обязательно:

```json
"confidence": {
  "overall": 0.0,
  "text": 0.0,
  "layout": 0.0,
  "structure": 0.0
}
```

### Rule 5: Координаты должны быть нормализованы

Все `bbox` должны быть в координатах страницы:

```text
origin: top_left
x grows right
y grows down
bbox: [x0, y0, x1, y1]
```

### Rule 6: Сохранять несколько представлений content

Для text-like элементов:

```json
"content": {
  "text": "plain text",
  "html": "<p>HTML</p>",
  "markdown": "Markdown",
  "normalized_text": "normalized for search",
  "raw": "original source fragment"
}
```

### Rule 7: Не хранить большие binary данные в JSON

В JSON не должно быть base64 изображений.

Использовать assets:

```json
{
  "asset_id": "asset_img_1",
  "path": "assets/images/image_1.jpg"
}
```

## 2. PDF rules

### PDF classification

PDF page can be:

```text
digital
scanned
hybrid
empty
corrupted
```

### Digital PDF

Если есть text layer:

- извлечь text spans;
- извлечь font size/style;
- извлечь bbox;
- извлечь images;
- извлечь links/annotations;
- попытаться извлечь tables.

### Scanned PDF

Если text layer отсутствует или содержит мусор:

- render page;
- layout detection;
- OCR;
- table detection;
- formula detection.

### Hybrid PDF

Если есть и text layer, и image regions:

- native extraction;
- render page;
- detect image regions;
- OCR only suspicious image regions;
- merge.

## 3. DOCX rules

DOCX should be parsed as OOXML.

Extract:

- paragraphs;
- runs;
- headings;
- lists;
- tables;
- images;
- footnotes;
- endnotes;
- comments;
- headers;
- footers;
- styles.

If exact bbox is required:

- convert/render to PDF or page image;
- map extracted structure to rendered layout if possible.

Embedded images should be analyzed:

```text
if image contains text:
  run OCR on image
```

## 4. HTML rules

HTML parser should extract:

- h1-h6 as heading;
- p as text;
- ul/ol as list;
- li as list item;
- table as table;
- img as image asset;
- blockquote as blockquote;
- pre/code as code;
- math/mathml/script type math as formula if detected.

HTML content must be sanitized if it will be rendered in UI.

## 5. Markdown rules

Markdown parser should preserve:

- headings;
- paragraphs;
- lists;
- code fences;
- blockquotes;
- tables;
- images;
- links.

Markdown has no real bbox.

Options:

1. synthetic bbox;
2. render to HTML and browser layout;
3. no bbox, only logical order.

## 6. Image rules

For image input:

- decode;
- apply EXIF orientation;
- normalize color;
- optionally deskew;
- optionally denoise;
- run layout detection;
- run OCR;
- extract tables/formulas if detected.

If image is a photo of a document:

- perform perspective correction if possible.

## 7. PPTX rules

Each slide becomes a page:

```json
"page_type": "slide"
```

Extract:

- text boxes;
- shapes;
- images;
- tables;
- speaker notes;
- charts if possible.

Images/screenshots in slides may require OCR.

Reading order should use visual layout, not only XML order.

## 8. XLSX rules

Each sheet becomes a page or logical page:

```json
"page_type": "sheet"
```

Extract:

- cells;
- formulas;
- merged cells;
- comments;
- charts;
- images;
- tables/ranges.

For large sheets:

- create multiple chunks;
- avoid dumping millions of cells into one element;
- store sheet ranges separately if needed.

## 9. TXT rules

TXT has no visual layout.

Actions:

- detect encoding;
- normalize newlines;
- detect headings;
- detect lists;
- detect code-like blocks;
- detect simple tables;
- create synthetic pages.

## 10. Error handling rules

Never fail the whole document if only one page failed.

Use partial success:

```json
"errors": [
  {
    "scope": "page",
    "page_number": 5,
    "code": "OCR_TIMEOUT",
    "message": "OCR timed out for page 5"
  }
]
```

Document status can be:

```text
ok
partial
failed
```