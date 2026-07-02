# MODEL ROUTING

Маршрутизация выполняется по шагам:

1. File classification.
2. Native extraction.
3. Domain detection.
4. Model profile routing.
5. OCR/layout/table/formula/legal/book stages.
6. Slow-path decision.
7. Запись model_outputs/domain_profile в model.extra.

## Юридический PDF

native PDF text
-> PP-OCRv6 selective OCR if needed
-> Surya/Docling layout
-> table extraction
-> GLiNER/rule legal extraction
-> USER-bge-m3 embeddings
-> slow path if key fields missing

## Скан договора

render
-> PP-OCRv6 det/rec
-> layout
-> scanned table detector
-> legal extraction
-> PaddleOCR-VL-1.6 if low confidence

## Судебный акт

native text
-> sections
-> court/case/date/party extraction
-> citations

## Художественная книга

OCR/native text
-> remove headers/footers
-> dehyphenation
-> paragraph reconstruction
-> chapter detection
-> footnotes

## Историческая книга

image preprocessing
-> Kraken/eScriptorium/Calamari
-> historical orthography detection
-> dehyphenation

## Научный документ

native/OCR
-> layout
-> tables
-> formulas
-> pix2tex/Pix2Text/PaddleOCR-VL fallback
