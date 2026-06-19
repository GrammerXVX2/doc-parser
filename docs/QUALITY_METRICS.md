# Quality Metrics

## Поля quality report

- document_id
- format
- language
- pages/elements/chars/words
- tables/images/formulas/ocr_elements
- warnings/errors/empty_pages
- duplicate_text_ratio
- low_confidence_ocr_ratio
- chunk_count

## Duplicate ratio

duplicate_text_ratio оценивает долю дублирующихся текстовых блоков:

- exact duplicates
- near-duplicates (через text similarity)

Используется для выявления регрессий native+OCR duplication.

## Low confidence OCR ratio

Доля OCR элементов с overall confidence < 0.5.

## Интерпретация warnings/errors

- warnings: частично деградировавшая обработка
- errors: критические сбои пайплайна

Рекомендуется отслеживать тренды по документам одного класса.
