# OCR BACKENDS

## Быстрый путь

- paddleocr_ppocrv6_medium: основной backend для ru/en.
- surya_ocr: fallback OCR backend.
- mock_ocr: безопасный fallback для dev/local окружений.

## Исторические документы

- kraken: primary historical OCR backend.
- escriptorium/calamari: альтернативы.
- paddleocr_ppocrv6_medium: fallback для совместимости.

## Slow path VLM

- paddleocr_vl_1_6
- qwen3_vl
- granite_docling_258m

Slow path может быть только decision-level на MVP этапе (без реального исполнения).
