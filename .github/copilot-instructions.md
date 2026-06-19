# Задача: Dev launch fixes для `GrammerXVX2/doc-parser`

## Контекст

Репозиторий: `GrammerXVX2/doc-parser`.

Система уже содержит CLI/API/job queue/storage/security/pipeline skeleton. Сейчас планируется запустить её в dev-режиме для 3 разработчиков, чтобы они могли загружать документы через API и получать:

```text
model.json
markdown.md
plain_text.txt
assets/
```

По аудиту обнаружены критичные нюансы, которые нужно исправить перед запуском.

---

# Главная цель

Подготовить проект к безопасному и удобному dev-запуску для 3 разработчиков.

После выполнения должно работать:

```bash
cargo run -- serve --config configs/profiles/dev_team.jsonc
```

И сценарий:

```bash
curl -F "file=@testdata/ru/sample_ru.html" \
     -F "language=ru" \
     -F "extract_tables=true" \
     -F "table_chunks=true" \
     http://127.0.0.1:8080/v1/documents
```

должен возвращать `job_id` и `document_id`.

После завершения job должны открываться:

```bash
GET /v1/jobs/{job_id}
GET /v1/documents/{document_id}/model
GET /v1/documents/{document_id}/markdown
GET /v1/documents/{document_id}/text
```

---

# P0. Исправить `document_id` mismatch в `JobWorker`

## Проблема

В `src/jobs/worker.rs` worker получает `job.document_id`, но после `run_pipeline` модель может иметь собственный `model.document_id`.

Сейчас output пишется через:

```rust
write_document_outputs(&model, &self.output_dir, true)?;
```

А `write_document_outputs` пишет в:

```text
output_dir / model.document_id / model.json
```

API же ищет результат по:

```text
output_dir / job.document_id / model.json
```

Из-за этого возможен сценарий:

```text
POST /v1/documents -> document_id = doc_abc
GET /v1/jobs/job_xyz -> completed
GET /v1/documents/doc_abc/model -> DOCUMENT_NOT_FOUND
```

## Требование

В `src/jobs/worker.rs` перед `write_document_outputs` обязательно выставить:

```rust
model.document_id = job.document_id.clone();
model.job_id = Some(job.job_id.clone());
```

Примерно здесь:

```rust
let (_classification, mut model) = run_pipeline(&job.input_path, &context)?;

if model.pages.len() > self.max_pages_per_document {
    return Err(anyhow::anyhow!("MAX_PAGES_LIMIT_EXCEEDED"));
}

model.document_id = job.document_id.clone();
model.job_id = Some(job.job_id.clone());

write_document_outputs(&model, &self.output_dir, true)?;
```

## Тест

Добавить/обновить тест, который проверяет:

1. Создаётся job с `document_id = doc_test`.
2. Worker обрабатывает input.
3. Output появляется именно в:

```text
output/doc_test/model.json
```

4. Внутри `model.json`:

```json
"document_id": "doc_test"
```

5. API `GET /v1/documents/doc_test/model` находит результат.

---

# P0. Добавить dev-team service profile

## Создать файл

```text
configs/profiles/dev_team.jsonc
```

## Содержимое

```jsonc
{
  "service": {
    "enabled": true,
    "host": "0.0.0.0",
    "port": 8080,
    "locale": "ru",
    "default_language": "ru",
    "max_concurrent_jobs": 2,
    "job_queue_capacity": 30
  },
  "storage": {
    "backend": "local",
    "input_dir": "data/input",
    "output_dir": "data/output",
    "metadata_backend": "local_json",
    "object_store_backend": "local"
  },
  "security": {
    "max_file_size_mb": 100,
    "max_pages_per_document": 300,
    "max_extracted_assets_mb": 512,
    "max_image_width_px": 8000,
    "max_image_height_px": 8000,
    "max_archive_entries": 3000,
    "max_archive_total_uncompressed_mb": 1024,
    "max_processing_time_sec": 300,
    "allow_external_converters": false,
    "allow_network_for_converters": false
  },
  "observability": {
    "tracing_enabled": true,
    "metrics_enabled": true,
    "prometheus_enabled": true,
    "prometheus_path": "/metrics"
  },
  "auth": {
    "enabled": false,
    "dev_token_env": "DOC_PARSER_DEV_TOKEN"
  }
}
```

## Почему

Для 3 разработчиков нельзя использовать только in-memory metadata, потому что после рестарта сервера все job statuses потеряются.

`metadata_backend` должен быть:

```jsonc
"local_json"
```

---

# P0. Исправить Docker/dev запуск

## Проблема

Сейчас `docker-compose.yml` запускает one-shot parse:

```yaml
command: ["cargo", "run", "--", "parse", "testdata/sample.html", "--output", "output"]
```

Для dev-команды нужен API server.

## Требование

Создать отдельный файл:

```text
docker-compose.dev.yml
```

Сервис должен запускать:

```bash
cargo run -- serve --config configs/profiles/dev_team.jsonc
```

## Содержимое

```yaml
services:
  document_parser:
    build:
      context: .
      dockerfile: docker/document-parser.Dockerfile
    image: document_parser:dev
    container_name: document_parser_dev
    environment:
      RUST_LOG: info
    volumes:
      - ./:/workspace
    working_dir: /workspace
    command: ["cargo", "run", "--", "serve", "--config", "configs/profiles/dev_team.jsonc"]
    ports:
      - "8080:8080"
```

## Важно

Не подключать `ocr_service` и GPU в dev compose по умолчанию.

GPU/OCR compose можно оставить отдельно для будущего.

---

# P0. Добавить dev smoke script

## Создать файл

```text
scripts/dev_smoke_api.sh
```

## Требования

Скрипт должен:

1. Проверить `/healthz`.
2. Проверить `/readyz`.
3. Загрузить тестовый документ.
4. Получить `job_id` и `document_id`.
5. Poll job status до:

```text
completed | partial | failed
```

6. Если статус:

```text
completed | partial
```

проверить:

```text
/v1/documents/{document_id}/model
/v1/documents/{document_id}/markdown
/v1/documents/{document_id}/text
```

7. Вернуть non-zero exit code при ошибке.

## Пример запуска

```bash
scripts/dev_smoke_api.sh http://127.0.0.1:8080 testdata/ru/sample_ru.html
```

Если аргументы не переданы:

```bash
BASE_URL=http://127.0.0.1:8080
INPUT_FILE=testdata/ru/sample_ru.html
```

## Dependencies

Можно использовать:

```text
bash
curl
python3
```

Если не хочется Python — использовать `jq`, но Python предпочтительнее, потому что `jq` может быть не установлен.

---

# P1. Добавить optional dev API token middleware

## Цель

Если API будет доступен не только локально, нужен простой dev-token.

## Config

В `ServiceProfile` добавить optional auth config:

```rust
pub struct AuthProfile {
    pub enabled: bool,
    pub dev_token_env: String,
}
```

Config:

```jsonc
"auth": {
  "enabled": false,
  "dev_token_env": "DOC_PARSER_DEV_TOKEN"
}
```

## Поведение

Если:

```text
auth.enabled = false
```

middleware ничего не требует.

Если:

```text
auth.enabled = true
```

API должен требовать header:

```http
Authorization: Bearer <token>
```

Token брать из env var:

```text
DOC_PARSER_DEV_TOKEN
```

Если env var отсутствует при `enabled=true`:

```text
readyz should be failed/degraded
requests should return 500/503 with AUTH_TOKEN_NOT_CONFIGURED
```

## Какие endpoints можно оставить public

```text
GET /healthz
GET /readyz
GET /metrics optional
```

Для dev можно защитить всё, кроме `healthz`/`readyz`.

## Ошибка

```json
{
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Необходим корректный Authorization Bearer token.",
    "recoverable": false,
    "details": {}
  }
}
```

## Тесты

Добавить tests:

```text
auth disabled -> upload works without token
auth enabled without token -> 401
auth enabled wrong token -> 401
auth enabled correct token -> upload accepted
```

Если middleware слишком большой для одного PR — можно сделать config field и тесты пропустить, но желательно реализовать.

---

# P1. Добавить документацию для dev-запуска

## Создать файл

```text
docs/DEV_LAUNCH.md
```

Документ должен быть на русском.

## Описать

```text
1. Как запустить локально через cargo.
2. Как запустить через docker compose.
3. Как проверить health/ready.
4. Как загрузить документ через curl.
5. Как проверить job status.
6. Как получить model/markdown/text.
7. Где лежат input/output/metadata.
8. Как очистить dev data.
9. Какие форматы разрешены в первом dev-цикле.
10. Что нельзя коммитить в репозиторий.
```

## Важное предупреждение

Добавить явно:

```text
Запрещено коммитить реальные юридические документы, персональные данные,
договоры, сканы паспортов, финансовые документы и OCR/debug artifacts.
```

## Разрешённые форматы первого dev-цикла

```text
.html
.md
.txt
.pdf
.png
.jpg
.jpeg
.docx
.xlsx
```

## Форматы второго dev-цикла

```text
.pptx
.rtf
.doc
.tiff
.webp
```

---

# P1. Добавить cleanup script

## Создать файл

```text
scripts/dev_clean_data.sh
```

## Поведение

Удалять:

```text
data/input/*
data/output/*
data/metadata/jobs/*
```

Но сохранять `.gitkeep`, если они есть.

Пример:

```bash
#!/usr/bin/env bash
set -euo pipefail

rm -rf data/input/* data/output/* data/metadata/jobs/*
mkdir -p data/input data/output data/metadata/jobs
touch data/input/.gitkeep data/output/.gitkeep data/metadata/jobs/.gitkeep

echo "Dev data cleaned."
```

---

# P1. Уменьшить риск memory spike на dev upload

## Проблема

`upload_document` читает весь файл в память:

```rust
field.bytes().await?.to_vec()
```

Для dev это допустимо, но лимит `512 MB` слишком большой.

## Требование

В `dev_team.jsonc` поставить:

```jsonc
"max_file_size_mb": 100
```

Также в `docs/DEV_LAUNCH.md` указать:

```text
На dev upload файл читается в память целиком, поэтому не загружайте большие архивы/сканы.
```

Streaming upload пока не реализовывать.

---

# P1. External converters disabled by default in dev

В `dev_team.jsonc` должно быть:

```jsonc
"allow_external_converters": false
```

В docs объяснить:

```text
RTF/DOC и некоторые fallback-конверсии могут не работать в dev_team profile,
потому что внешние конвертеры выключены для безопасности и стабильности.
```

---

# P1. Добавить простой список dev curl-команд

В `docs/DEV_LAUNCH.md` добавить:

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

---

# P2. Добавить list endpoints, если быстро

Опционально, если не сильно увеличит PR:

```text
GET /v1/jobs
GET /v1/documents
```

Для dev-команды это удобно.

MVP response:

```json
{
  "jobs": [
    {
      "job_id": "...",
      "document_id": "...",
      "status": "completed",
      "created_at": "...",
      "updated_at": "..."
    }
  ]
}
```

Если `MetadataStore` не умеет list — добавить только для:

```text
InMemoryMetadataStore
LocalJsonMetadataStore
```

Если это слишком большой объём — не делать в этом PR.

---

# Тесты, которые обязательно добавить/обновить

## 1. Worker document_id test

Файл:

```text
tests/job_worker_output_tests.rs
```

Проверить:

```text
worker writes output to job.document_id directory
model.json contains job.document_id
```

---

## 2. Dev profile loads

Файл:

```text
tests/dev_profile_tests.rs
```

Проверить:

```text
configs/profiles/dev_team.jsonc loads
metadata_backend == local_json
max_file_size_mb <= 100
allow_external_converters == false
locale == ru
```

---

## 3. API end-to-end test

Если есть test harness для API:

```text
upload -> job complete -> get model by returned document_id
```

Если полный worker test тяжёлый, можно сделать более простой test вокруг worker/output.

---

## 4. Auth tests

Если реализован optional token middleware:

```text
auth disabled accepts request
auth enabled rejects missing/wrong token
auth enabled accepts correct token
```

---

## 5. Script presence test optional

Не обязательно, но можно проверить, что файлы существуют:

```text
scripts/dev_smoke_api.sh
scripts/dev_clean_data.sh
docker-compose.dev.yml
docs/DEV_LAUNCH.md
```

---

# Acceptance criteria

Задача считается выполненной, если:

1. `model.document_id` в worker принудительно синхронизируется с `job.document_id`.
2. После API upload result можно получить по `document_id`, возвращенному в upload response.
3. Добавлен файл:

```text
configs/profiles/dev_team.jsonc
```

4. `dev_team.jsonc` использует:

```text
metadata_backend = local_json
max_file_size_mb = 100
max_pages_per_document = 300
allow_external_converters = false
locale = ru
```

5. Добавлен `docker-compose.dev.yml`, который запускает API server, а не one-shot parse.
6. Добавлен `scripts/dev_smoke_api.sh`.
7. Добавлен `scripts/dev_clean_data.sh`.
8. Добавлен `docs/DEV_LAUNCH.md`.
9. В docs явно написано, что нельзя коммитить реальные документы и персональные данные.
10. Если реализован auth:
    - config поддерживает `auth.enabled`;
    - `Authorization: Bearer` проверяется;
    - tests добавлены.
11. Все тесты проходят:

```bash
cargo test
```

12. Проект компилируется:

```bash
cargo check
```

13. Existing CLI parse mode не сломан:

```bash
cargo run -- parse testdata/ru/sample_ru.html --output output
```

14. Dev server запускается:

```bash
cargo run -- serve --config configs/profiles/dev_team.jsonc
```

15. Smoke script проходит:

```bash
scripts/dev_smoke_api.sh http://127.0.0.1:8080 testdata/ru/sample_ru.html
```

---

# Не делать в этом PR

Не нужно сейчас:

```text
real OCR model integration
PostgreSQL
S3
Kubernetes
streaming upload
full auth/users/roles
web UI
GPU/TensorRT/Triton
```

Главная цель — стабильный dev-запуск для 3 разработчиков.