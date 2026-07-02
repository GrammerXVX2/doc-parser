# BOOK EXTRACTION

MVP extraction для художественной и исторической литературы.

## Возможности

- chapter detection: Глава 1, ГЛАВА I, Часть первая, I., 1.
- footnote detection
- dehyphenation: сло-\nво -> слово
- historical orthography detection: ѣ, і, ѳ, ѵ

## Ограничения MVP

Dehyphenation не должен затрагивать:

- code blocks
- URLs
- formulas
- tables

## Output

Результат сохраняется в model.extra.book.
