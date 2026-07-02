# MODEL STACK

Этот документ описывает расширенный model stack для OCR и document parsing.

## OCR и Parsing

- PP-OCRv6 medium det/rec: быстрый OCR backend по умолчанию для ru/en.
- Docling: structured parsing для сложных PDF и mixed layouts.
- Surya: OCR/layout/table fallback.
- PaddleOCR-VL-1.6: slow-path VLM backend для сложных страниц.
- Granite Docling 258M: альтернативный VLM/structured backend.
- Qwen3-VL: альтернативный slow-path VLM backend.

## Legal

- GLiNER v2.5 (large/medium/small): NER для юридических сущностей.
- Rule-based legal extractor: обязательный fallback без внешних зависимостей.
- USER-bge-m3 и BGE-M3: embeddings для юридического семантического слоя.

## Historical Books

- Kraken/eScriptorium/Calamari: historical OCR стек.
- PP-OCRv6 и Surya: fallback для сложных исторических сканов.

## Конфигурация

Полный стек и профили задаются в configs/model_stack.config.jsonc.
