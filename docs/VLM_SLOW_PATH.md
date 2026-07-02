# VLM SLOW PATH

Slow path запускается по решению роутера и quality-триггерам.

## Triggers

- OCR confidence ниже threshold
- layout confidence ниже threshold
- placeholder table/formula
- legal profile с отсутствующими parties/dates/identifiers
- профиль legal_high_accuracy
- user forced slow path

## Backends

- paddleocr_vl_1_6 (primary)
- qwen3_vl (alternative)
- granite_docling_258m (alternative)

## MVP поведение

На текущем этапе сохраняется решение (decision) и метаданные в model_outputs.slow_path.
Фактическое исполнение backend может быть отключено.
