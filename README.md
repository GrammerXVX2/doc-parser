# document_parser

Универсальный parser документов в канонический `DocumentModel` с русским поведением по умолчанию.

## Что делает проект

- Разбирает документы разных форматов в единый `model.json`.
- Сохраняет дополнительные артефакты (assets), markdown и plain text.
- Поддерживает pipeline с native extraction, OCR, layout/table/formula этапами.
- Предоставляет CLI и HTTP API режимы.

## Поддерживаемые форматы

- HTML
- Markdown
- TXT
- PDF
- Image (png/jpg/tiff/webp и др.)
- DOCX
- XLSX
- PPTX
- RTF
- DOC

## Quick Start CLI

```bash
cargo run -- parse testdata/ru/sample_ru.html --output output
```

Дополнительные команды:

```bash
cargo run -- bench --input benchmarks/datasets/small_ru --output benchmarks/reports/local --profile configs/profiles/benchmark.jsonc
cargo run -- quality --input output/<document_id>/model.json
cargo run -- doctor
```

## Quick Start API

```bash
cargo run -- serve --config configs/profiles/api.jsonc
```
Командный dev-профиль (3 разработчика):

```bash
cargo run -- serve --config configs/profiles/dev_team.jsonc
```

Проверка:

```bash
curl -s http://127.0.0.1:8080/healthz
curl -s http://127.0.0.1:8080/readyz
curl -s http://127.0.0.1:8080/metrics
```

## Структура output

```text
output/
  <document_id>/
    model.json
    markdown.md
    plain_text.txt
    assets/
```

## Russian-first поведение

- По умолчанию язык и сообщения ориентированы на русский.
- Машинные коды ошибок/предупреждений сохраняются на английском uppercase.

## Документация

- [docs/README.md](docs/README.md)
- [docs/PRODUCTION.md](docs/PRODUCTION.md)
- [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md)
- [docs/SECURITY.md](docs/SECURITY.md)
- [docs/OBSERVABILITY.md](docs/OBSERVABILITY.md)
- [docs/RUNBOOK.md](docs/RUNBOOK.md)
- [docs/REGRESSION_TESTING.md](docs/REGRESSION_TESTING.md)
- [docs/QUALITY_METRICS.md](docs/QUALITY_METRICS.md)
- [docs/RELEASE_CHECKLIST.md](docs/RELEASE_CHECKLIST.md)

## Ограничения

- Нет cloud-specific Kubernetes manifests в базовой поставке.
- Нет production-ready auth/billing/multitenancy из коробки.
- Реальные OCR/model stacks требуют отдельной интеграции и калибровки.
