# LEGAL EXTRACTION

MVP legal extraction выполняется rule-based способом.

## Извлекаемые поля

- document_type
- parties
- dates
- amounts
- identifiers: ИНН/КПП/ОГРН/номер договора
- clauses
- risks
- citations

## Backend Strategy

- primary: GLiNER v2.5 (large)
- fallback: medium -> small -> rule_based_legal_extractor

На текущем этапе GLiNER подключен как интеграционная точка; рабочий путь по умолчанию остается rule-based.

## Output

Результат сохраняется в model.extra.legal.
