
## Dev Launch

- [DEV_LAUNCH.md](DEV_LAUNCH.md)
# Universal Document Parsing Pipeline

## Назначение

Система предназначена для разбора разношёрстных документов в единую каноническую структуру `model.json`.

Поддерживаемые входные форматы:

- PDF:
  - цифровой PDF с текстовым слоем;
  - сканированный PDF;
  - гибридный PDF: текст + картинки + сканы.
- DOCX / DOC:
  - текст;
  - таблицы;
  - изображения;
  - embedded scans;
  - формулы;
  - колонтитулы.
- Markdown:
  - текст;
  - таблицы;
  - code blocks;
  - картинки;
  - embedded HTML.
- HTML:
  - DOM-структура;
  - headings;
  - paragraphs;
  - lists;
  - tables;
  - images;
  - formulas;
  - code blocks.
- RTF.
- Images:
  - JPG;
  - PNG;
  - TIFF;
  - WEBP;
  - scans.
- PPTX:
  - slides;
  - text boxes;
  - images;
  - tables;
  - screenshots.
- TXT.
- XLSX:
  - sheets;
  - cells;
  - formulas;
  - tables;
  - charts;
  - embedded images.

## Главная идея

Система не должна быть просто OCR-пайплайном.

Она должна быть **универсальным Document Understanding Pipeline**:

```text
Input document
  ↓
File detection
  ↓
Format-specific native extraction
  ↓
Rendering when needed
  ↓
Layout analysis
  ↓
Selective OCR
  ↓
Table / formula / image understanding
  ↓
Native + OCR merge
  ↓
Reading order reconstruction
  ↓
Chunking for RAG / LLM
  ↓
model.json + assets
```

## Основной результат

На выходе система должна создавать:

```text
output/
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
```

## Основные принципы

1. Сначала извлекать native-структуру, если она есть.
2. OCR использовать выборочно, только там, где нет надёжного текста.
3. Не дублировать native text и OCR text.
4. Любой элемент должен иметь provenance — источник происхождения.
5. Любой OCR или ML-результат должен иметь confidence.
6. Все координаты должны быть нормализованы в единую систему.
7. Для RAG нужно создавать semantic chunks, а не резать текст вслепую.
8. Все тяжёлые файлы — изображения, рендеры страниц, таблицы — хранить как assets, а в JSON класть ссылки.
9. Pipeline должен быть устойчив к битым, защищённым и частично повреждённым документам.
10. Все этапы должны логироваться и измеряться метриками.

## Индекс эксплуатационной документации

- [PRODUCTION.md](PRODUCTION.md)
- [DEPLOYMENT.md](DEPLOYMENT.md)
- [SECURITY.md](SECURITY.md)
- [OBSERVABILITY.md](OBSERVABILITY.md)
- [RUNBOOK.md](RUNBOOK.md)
- [DEV_LAUNCH.md](DEV_LAUNCH.md)
- [REGRESSION_TESTING.md](REGRESSION_TESTING.md)
- [QUALITY_METRICS.md](QUALITY_METRICS.md)
- [RELEASE_CHECKLIST.md](RELEASE_CHECKLIST.md)