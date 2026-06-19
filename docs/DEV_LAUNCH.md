# DEV Launch Guide

Документ описывает минимально безопасный запуск `doc-parser` в dev-режиме для команды из 3 разработчиков.

## 1. Локальный запуск через cargo

```bash
cargo run -- serve --config configs/profiles/dev_team.jsonc
```

Профиль `dev_team` использует:

- `metadata_backend = local_json` (статусы job сохраняются между рестартами)
- `max_file_size_mb = 100`
- `max_pages_per_document = 300`
- `allow_external_converters = false`
- `locale = ru`

## 2. Запуск через Docker Compose

```bash
docker compose -f docker-compose.dev.yml up --build
```

`docker-compose.dev.yml` запускает API server командой:

```bash
cargo run -- serve --config configs/profiles/dev_team.jsonc
```

## 3. Проверка health/ready

```bash
curl -s http://127.0.0.1:8080/healthz
curl -s http://127.0.0.1:8080/readyz
```

## 4. Загрузка документа через curl

```bash
curl -s \
  -F "file=@testdata/ru/sample_ru.html" \
  -F "language=ru" \
  -F "extract_tables=true" \
  -F "table_chunks=true" \
  http://127.0.0.1:8080/v1/documents
```

Ответ должен содержать `job_id` и `document_id`.

## 5. Проверка статуса job

```bash
curl -s http://127.0.0.1:8080/v1/jobs/<job_id>
```

## 6. Получение результатов

```bash
curl -s http://127.0.0.1:8080/v1/documents/<document_id>/model
curl -s http://127.0.0.1:8080/v1/documents/<document_id>/markdown
curl -s http://127.0.0.1:8080/v1/documents/<document_id>/text
```

## 7. Где лежат данные

- Input uploads: `data/input`
- Output documents: `data/output`
- Metadata jobs: `data/metadata/jobs`

## 8. Очистка dev-данных

```bash
bash scripts/dev_clean_data.sh
```

Smoke-проверка API:

```bash
bash scripts/dev_smoke_api.sh http://127.0.0.1:8080 testdata/ru/sample_ru.html
```

Если аргументы не переданы, используются:

- `BASE_URL=http://127.0.0.1:8080`
- `INPUT_FILE=testdata/ru/sample_ru.html`

## 9. Разрешенные форматы

Первый dev-цикл:

- `.html`
- `.md`
- `.txt`
- `.pdf`
- `.png`
- `.jpg`
- `.jpeg`
- `.docx`
- `.xlsx`

Второй dev-цикл:

- `.pptx`
- `.rtf`
- `.doc`
- `.tiff`
- `.webp`

## 10. Что нельзя коммитить

Запрещено коммитить реальные юридические документы, персональные данные,
договоры, сканы паспортов, финансовые документы и OCR/debug artifacts.

Дополнительно:

- На dev upload файл читается в память целиком, поэтому не загружайте большие архивы/сканы.
- В `dev_team` профиле внешние конвертеры по умолчанию выключены (`allow_external_converters=false`), поэтому RTF/DOC и часть fallback-конверсий могут не работать.

## Быстрый набор dev-команд

```bash
# Запуск
cargo run -- serve --config configs/profiles/dev_team.jsonc

# Health
curl -s http://127.0.0.1:8080/healthz

# Ready
curl -s http://127.0.0.1:8080/readyz

# Upload
curl -s \
  -F "file=@testdata/ru/sample_ru.html" \
  -F "language=ru" \
  -F "extract_tables=true" \
  -F "table_chunks=true" \
  http://127.0.0.1:8080/v1/documents

# Job
curl -s http://127.0.0.1:8080/v1/jobs/<job_id>

# Model
curl -s http://127.0.0.1:8080/v1/documents/<document_id>/model

# Markdown
curl -s http://127.0.0.1:8080/v1/documents/<document_id>/markdown

# Text
curl -s http://127.0.0.1:8080/v1/documents/<document_id>/text
```
# DEV Launch Guide

## 1) Локальный запуск через cargo

```bash
cargo run -- serve --config configs/profiles/dev_team.jsonc
```

Профиль `dev_team` рассчитан на команду из 3 разработчиков и хранит job metadata в локальных JSON-файлах.

## 2) Запуск через docker compose

```bash
docker compose -f docker-compose.dev.yml up --build
```

Dev compose поднимает только API сервис без OCR/GPU стека по умолчанию.

## 3) Проверка health/ready

```bash
curl -s http://127.0.0.1:8080/healthz
curl -s http://127.0.0.1:8080/readyz
```

## 4) Загрузка документа

```bash
curl -s \
  -F "file=@testdata/ru/sample_ru.html" \
  -F "language=ru" \
  -F "extract_tables=true" \
  -F "table_chunks=true" \
  http://127.0.0.1:8080/v1/documents
```

Ожидаемый ответ содержит `job_id` и `document_id`.

## 5) Проверка статуса job

```bash
curl -s http://127.0.0.1:8080/v1/jobs/<job_id>
```

## 6) Получение результатов

```bash
curl -s http://127.0.0.1:8080/v1/documents/<document_id>/model
curl -s http://127.0.0.1:8080/v1/documents/<document_id>/markdown
curl -s http://127.0.0.1:8080/v1/documents/<document_id>/text
```

## 7) Где лежат данные

- Входные файлы: `data/input`
- Результаты: `data/output/<document_id>/`
- Метаданные jobs: `data/metadata/jobs`

## 8) Очистка dev-данных

```bash
scripts/dev_clean_data.sh
```

## 9) Разрешенные форматы

Первый dev-цикл:

- `.html`
- `.md`
- `.txt`
- `.pdf`
- `.png`
- `.jpg`
- `.jpeg`
- `.docx`
- `.xlsx`

Второй dev-цикл:

- `.pptx`
- `.rtf`
- `.doc`
- `.tiff`
- `.webp`

## 10) Что нельзя коммитить в репозиторий

Запрещено коммитить реальные юридические документы, персональные данные, договоры, сканы паспортов, финансовые документы и OCR/debug artifacts.

## Ограничения и безопасность

- В `dev_team` профиле `max_file_size_mb=100`.
- На dev upload файл читается в память целиком, поэтому не загружайте большие архивы/сканы.
- Внешние конвертеры по умолчанию выключены (`allow_external_converters=false`).
- RTF/DOC и некоторые fallback-конверсии могут не работать в `dev_team` profile, потому что внешние конвертеры выключены для безопасности и стабильности.

## Optional dev token

Если в `configs/profiles/dev_team.jsonc` включить:

```json
"auth": {
  "enabled": true,
  "dev_token_env": "DOC_PARSER_DEV_TOKEN"
}
```

то для защищенных API endpoints потребуется заголовок:

```text
Authorization: Bearer <token>
```

Токен берется из env переменной `DOC_PARSER_DEV_TOKEN`.

## Быстрый smoke

```bash
scripts/dev_smoke_api.sh http://127.0.0.1:8080 testdata/ru/sample_ru.html
```

## Набор команд для dev

```bash
# Запуск
cargo run -- serve --config configs/profiles/dev_team.jsonc

# Health
curl -s http://127.0.0.1:8080/healthz

# Ready
curl -s http://127.0.0.1:8080/readyz

# Upload
curl -s \
  -F "file=@testdata/ru/sample_ru.html" \
  -F "language=ru" \
  -F "extract_tables=true" \
  -F "table_chunks=true" \
  http://127.0.0.1:8080/v1/documents

# Job
curl -s http://127.0.0.1:8080/v1/jobs/<job_id>

# Model
curl -s http://127.0.0.1:8080/v1/documents/<document_id>/model

# Markdown
curl -s http://127.0.0.1:8080/v1/documents/<document_id>/markdown

# Text
curl -s http://127.0.0.1:8080/v1/documents/<document_id>/text
```
