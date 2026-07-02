# Задача: Real Model Runtime Phase 1 — подготовить развёртывание и подключить первые реальные backends

## Репозиторий

```text
GrammerXVX2/doc-parser
```

## Контекст

В проекте уже должна быть архитектура model stack:

```text
configs/model_stack.config.jsonc
model routing
domain detection
model_outputs
domain_profile
legal/book outputs
slow_path decision
mock/fixture model backends
```

Теперь нужно перейти от mock/fixture backends к **первому реальному dev-развёртыванию моделей**.

---

# Главная цель

Подготовить runtime-интеграцию и подключить первые реальные модели/backends:

```text
1. PaddleOCR PP-OCRv6 medium det/rec
2. Surya OCR/layout/table
3. Docling structured document parsing
```

На этом этапе НЕ подключать пока:

```text
PaddleOCR-VL-1.6
Qwen3-VL
Granite Docling 258M
GLiNER
BGE-M3 / USER-bge-m3
pix2tex / Pix2Text
Kraken / eScriptorium / Calamari
```

Но оставить для них placeholders/config.

---

# Почему только эти backends

## PP-OCRv6

Это быстрый OCR fast path:

```text
scanned PDF
image documents
embedded images
hybrid PDFs
```

## Surya

Это layout/OCR/table helper:

```text
layout analysis
reading order
OCR fallback
table detection/recognition helper
```

## Docling

Это structured document parser:

```text
complex PDFs
mixed documents
RAG-ready Markdown/JSON fallback
tables/layout/formulas where supported
```

Эти три дают максимальную пользу для dev-команды и являются фундаментом для тяжёлых VLM/Legal/Formula backends.

---

# Важное требование

В dev режиме real backend failures не должны ломать pipeline.

Если backend недоступен:

```text
- записать structured warning;
- использовать mock/fixture fallback;
- model.json должен создаться;
- API job не должен падать без необходимости.
```

---

# Архитектурный подход

Реальные модели подключать через один из типов backend:

```text
1. HTTP service backend
2. external command backend
3. local library backend where practical
```

Для dev предпочтительно:

```text
HTTP service backend
```

Почему:

```text
- Rust parser не тащит Python ML dependencies;
- модели можно обновлять независимо;
- проще Docker Compose;
- проще health checks;
- проще fallback.
```

---

# Новые директории

Добавить:

```text
model_services/
  paddleocr_service/
    Dockerfile
    requirements.txt
    app.py
    README.md

  surya_service/
    Dockerfile
    requirements.txt
    app.py
    README.md

  docling_service/
    Dockerfile
    requirements.txt
    app.py
    README.md

docker-compose.models.dev.yml

docs/
  MODEL_SERVICES_DEV.md
  REAL_MODEL_RUNTIME.md
```

---

# Этап 1. Обновить model_stack config для HTTP services

В `configs/model_stack.config.jsonc` обновить backends:

```jsonc
{
  "model_stack": {
    "global": {
      "enable_real_models": true,
      "fallback_to_mock": true,
      "fallback_to_fixture": true,
      "fail_on_missing_required_model": false
    },

    "backends": {
      "paddleocr_ppocrv6_medium": {
        "kind": "ocr",
        "enabled": true,
        "backend_type": "http",
        "required": false,
        "languages": ["ru", "en"],
        "detection_model": "PaddlePaddle/PP-OCRv6_medium_det",
        "recognition_model": "PaddlePaddle/PP-OCRv6_medium_rec",
        "url": "http://127.0.0.1:8101",
        "health_path": "/healthz",
        "ocr_path": "/v1/ocr",
        "timeout_sec": 120,
        "fallback_to_mock": true
      },

      "surya_ocr": {
        "kind": "ocr",
        "enabled": true,
        "backend_type": "http",
        "required": false,
        "languages": ["ru", "en"],
        "url": "http://127.0.0.1:8102",
        "health_path": "/healthz",
        "ocr_path": "/v1/ocr",
        "layout_path": "/v1/layout",
        "table_path": "/v1/tables",
        "timeout_sec": 120,
        "fallback_to_mock": true
      },

      "surya_layout": {
        "kind": "layout",
        "enabled": true,
        "backend_type": "http",
        "required": false,
        "url": "http://127.0.0.1:8102",
        "health_path": "/healthz",
        "layout_path": "/v1/layout",
        "timeout_sec": 120,
        "fallback_to_heuristic": true
      },

      "surya_table": {
        "kind": "table_structure",
        "enabled": true,
        "backend_type": "http",
        "required": false,
        "url": "http://127.0.0.1:8102",
        "health_path": "/healthz",
        "table_path": "/v1/tables",
        "timeout_sec": 120,
        "fallback_to_placeholder": true
      },

      "docling": {
        "kind": "structured_document_parse",
        "enabled": true,
        "backend_type": "http",
        "required": false,
        "url": "http://127.0.0.1:8103",
        "health_path": "/healthz",
        "parse_path": "/v1/parse",
        "timeout_sec": 300,
        "fallback_to_mock": true
      },

      "docling_layout": {
        "kind": "layout",
        "enabled": true,
        "backend_type": "http",
        "required": false,
        "url": "http://127.0.0.1:8103",
        "health_path": "/healthz",
        "layout_path": "/v1/layout",
        "timeout_sec": 300,
        "fallback_to_heuristic": true
      },

      "docling_tableformer": {
        "kind": "table_structure",
        "enabled": true,
        "backend_type": "http",
        "required": false,
        "url": "http://127.0.0.1:8103",
        "health_path": "/healthz",
        "table_path": "/v1/tables",
        "timeout_sec": 300,
        "fallback_to_placeholder": true
      }
    }
  }
}
```

---

# Этап 2. Добавить HTTP model backend client в Rust

Создать:

```text
src/models/backends/http.rs
```

Реализовать общий клиент:

```rust
pub struct HttpModelBackendClient {
    pub name: String,
    pub base_url: String,
    pub timeout: Duration,
}
```

Методы:

```rust
impl HttpModelBackendClient {
    pub async fn health_check(&self, health_path: &str) -> ModelBackendHealth;

    pub async fn post_json<TReq, TResp>(
        &self,
        path: &str,
        payload: &TReq
    ) -> anyhow::Result<TResp>
    where
        TReq: Serialize + ?Sized,
        TResp: DeserializeOwned;
}
```

Требования:

```text
- reqwest async client;
- timeout;
- structured errors;
- no panic;
- fallback hooks.
```

---

# Этап 3. Добавить shared HTTP schemas

Создать:

```text
src/models/backends/http_schemas.rs
```

Общие request/response:

```rust
pub struct OcrHttpRequest {
    pub document_id: String,
    pub page_number: u32,
    pub image_path: String,
    pub languages: Vec<String>,
    pub options: serde_json::Value,
}
```

```rust
pub struct OcrHttpResponse {
    pub backend: String,
    pub regions: Vec<OcrHttpRegion>,
    pub text: Option<String>,
    pub confidence: Option<f32>,
    pub metadata: serde_json::Value,
}
```

```rust
pub struct OcrHttpRegion {
    pub text: String,
    pub bbox: [f32; 4],
    pub confidence: f32,
    pub language: Option<String>,
}
```

```rust
pub struct LayoutHttpRequest {
    pub document_id: String,
    pub page_number: u32,
    pub image_path: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub options: serde_json::Value,
}
```

```rust
pub struct LayoutHttpResponse {
    pub backend: String,
    pub regions: Vec<LayoutHttpRegion>,
    pub metadata: serde_json::Value,
}
```

```rust
pub struct LayoutHttpRegion {
    pub region_type: String,
    pub bbox: [f32; 4],
    pub confidence: f32,
    pub text: Option<String>,
}
```

```rust
pub struct StructuredParseHttpRequest {
    pub document_id: String,
    pub input_path: String,
    pub format: String,
    pub language: String,
    pub options: serde_json::Value,
}
```

```rust
pub struct StructuredParseHttpResponse {
    pub backend: String,
    pub markdown: Option<String>,
    pub text: Option<String>,
    pub elements: serde_json::Value,
    pub tables: serde_json::Value,
    pub metadata: serde_json::Value,
    pub confidence: Option<f32>,
}
```

---

# Этап 4. Реализовать Rust adapters

Добавить:

```text
src/models/ocr/paddleocr.rs
src/models/ocr/surya.rs
src/models/layout/surya_layout.rs
src/models/structured/docling.rs
src/models/tables/surya_table.rs
src/models/tables/docling_tableformer.rs
```

## PaddleOCR adapter

```rust
pub struct PaddleOcrV6HttpBackend {
    client: HttpModelBackendClient,
    config: ModelBackendConfig,
}
```

Реализовать:

```rust
ExtendedOcrBackend for PaddleOcrV6HttpBackend
```

Поведение:

```text
call /v1/ocr
map OcrHttpResponse -> text_ocr Elements
provenance.tool = paddleocr_ppocrv6_medium
confidence from response
fallback to mock if unavailable
```

## Surya adapter

Реализовать:

```text
SuryaOcrHttpBackend
SuryaLayoutHttpBackend
SuryaTableHttpBackend
```

## Docling adapter

Реализовать:

```text
DoclingStructuredParseHttpBackend
DoclingLayoutHttpBackend
DoclingTableFormerHttpBackend
```

На первом этапе `DoclingStructuredParseHttpBackend` может:

```text
получать markdown/text/elements JSON;
нормализовать в DocumentModel extra или Elements;
если не удаётся — warning + fallback.
```

---

# Этап 5. Добавить model service health checks в `doctor`

Расширить:

```bash
cargo run -- doctor
```

Проверять:

```text
paddleocr service health
surya service health
docling service health
```

Если service недоступен:

```text
WARN, если fallback_to_mock=true
ERROR, если required=true
```

Пример вывода:

```text
OK: Конфигурация model stack загружена
WARN: PaddleOCR service недоступен, будет использован mock fallback
WARN: Surya service недоступен, будет использован heuristic fallback
WARN: Docling service недоступен, будет использован native/mock fallback
```

JSON output тоже обновить.

---

# Этап 6. Создать PaddleOCR service

Создать:

```text
model_services/paddleocr_service/
  Dockerfile
  requirements.txt
  app.py
  README.md
```

## requirements.txt

```text
fastapi
uvicorn[standard]
pillow
python-multipart
pydantic
numpy
```

Если сразу используем PaddleOCR Python package:

```text
paddleocr
paddlepaddle
```

Но если это слишком тяжело для CI/dev, сервис должен поддерживать mock mode.

## ENV

```text
PADDLEOCR_BACKEND=mock|real
PADDLEOCR_DEVICE=cpu|gpu
PADDLEOCR_LANG=ru,en
PADDLEOCR_DET_MODEL=PaddlePaddle/PP-OCRv6_medium_det
PADDLEOCR_REC_MODEL=PaddlePaddle/PP-OCRv6_medium_rec
```

## Endpoints

```text
GET /healthz
POST /v1/ocr
```

## `/v1/ocr` request

```json
{
  "document_id": "doc_1",
  "page_number": 1,
  "image_path": "/workspace/data/output/doc_1/assets/renders/page_1.png",
  "languages": ["ru", "en"],
  "options": {}
}
```

## `/v1/ocr` response

```json
{
  "backend": "paddleocr_ppocrv6_medium",
  "regions": [
    {
      "text": "Тестовый OCR текст",
      "bbox": [100, 100, 500, 140],
      "confidence": 0.95,
      "language": "ru"
    }
  ],
  "text": "Тестовый OCR текст",
  "confidence": 0.95,
  "metadata": {
    "mode": "mock"
  }
}
```

## Mock mode

Если real PaddleOCR не установлен или `PADDLEOCR_BACKEND=mock`, сервис должен возвращать deterministic OCR.

---

# Этап 7. Создать Surya service

Создать:

```text
model_services/surya_service/
  Dockerfile
  requirements.txt
  app.py
  README.md
```

## ENV

```text
SURYA_BACKEND=mock|real
SURYA_DEVICE=cpu|gpu
```

## Endpoints

```text
GET /healthz
POST /v1/ocr
POST /v1/layout
POST /v1/tables
```

## Mock responses

### `/v1/layout`

```json
{
  "backend": "surya_layout",
  "regions": [
    {
      "region_type": "title",
      "bbox": [50, 40, 900, 100],
      "confidence": 0.95,
      "text": null
    },
    {
      "region_type": "text",
      "bbox": [50, 120, 900, 500],
      "confidence": 0.90,
      "text": null
    }
  ],
  "metadata": {
    "mode": "mock"
  }
}
```

### `/v1/tables`

```json
{
  "backend": "surya_table",
  "regions": [
    {
      "region_type": "table",
      "bbox": [100, 300, 900, 700],
      "confidence": 0.88,
      "text": null
    }
  ],
  "metadata": {
    "mode": "mock"
  }
}
```

---

# Этап 8. Создать Docling service

Создать:

```text
model_services/docling_service/
  Dockerfile
  requirements.txt
  app.py
  README.md
```

## ENV

```text
DOCLING_BACKEND=mock|real
DOCLING_DEVICE=cpu|gpu
```

## Endpoints

```text
GET /healthz
POST /v1/parse
POST /v1/layout
POST /v1/tables
```

## `/v1/parse` response

```json
{
  "backend": "docling",
  "markdown": "# Документ\n\nТекст документа.",
  "text": "Документ\n\nТекст документа.",
  "elements": [],
  "tables": [],
  "metadata": {
    "mode": "mock"
  },
  "confidence": 0.9
}
```

Mock mode обязателен.

Real mode можно сделать best-effort, но не блокировать PR.

---

# Этап 9. Docker compose для моделей

Создать:

```text
docker-compose.models.dev.yml
```

Содержимое:

```yaml
services:
  paddleocr_service:
    build:
      context: .
      dockerfile: model_services/paddleocr_service/Dockerfile
    image: doc_parser_paddleocr_service:dev
    container_name: doc_parser_paddleocr_service
    environment:
      PADDLEOCR_BACKEND: mock
      PADDLEOCR_DEVICE: cpu
      PADDLEOCR_LANG: ru,en
    volumes:
      - ./:/workspace
    working_dir: /workspace
    ports:
      - "8101:8101"
    command: ["uvicorn", "model_services.paddleocr_service.app:app", "--host", "0.0.0.0", "--port", "8101"]

  surya_service:
    build:
      context: .
      dockerfile: model_services/surya_service/Dockerfile
    image: doc_parser_surya_service:dev
    container_name: doc_parser_surya_service
    environment:
      SURYA_BACKEND: mock
      SURYA_DEVICE: cpu
    volumes:
      - ./:/workspace
    working_dir: /workspace
    ports:
      - "8102:8102"
    command: ["uvicorn", "model_services.surya_service.app:app", "--host", "0.0.0.0", "--port", "8102"]

  docling_service:
    build:
      context: .
      dockerfile: model_services/docling_service/Dockerfile
    image: doc_parser_docling_service:dev
    container_name: doc_parser_docling_service
    environment:
      DOCLING_BACKEND: mock
      DOCLING_DEVICE: cpu
    volumes:
      - ./:/workspace
    working_dir: /workspace
    ports:
      - "8103:8103"
    command: ["uvicorn", "model_services.docling_service.app:app", "--host", "0.0.0.0", "--port", "8103"]
```

---

# Этап 10. Интеграция с dev API compose

Обновить или создать:

```text
docker-compose.full.dev.yml
```

Который поднимает:

```text
document_parser API
paddleocr_service
surya_service
docling_service
```

API service должен зависеть от model services:

```yaml
depends_on:
  - paddleocr_service
  - surya_service
  - docling_service
```

И использовать:

```text
configs/profiles/dev_team.jsonc
configs/model_stack.config.jsonc
```

---

# Этап 11. Runtime fallback behavior

Если HTTP service недоступен:

```text
- MODEL_BACKEND_NOT_AVAILABLE warning
- MODEL_BACKEND_FALLBACK_USED warning
- use mock backend
- continue pipeline
```

Если service вернул invalid response:

```text
- MODEL_BACKEND_RESPONSE_INVALID
- fallback
```

Если timeout:

```text
- MODEL_BACKEND_TIMEOUT
- fallback
```

Добавить warning codes:

```text
MODEL_BACKEND_TIMEOUT
MODEL_BACKEND_RESPONSE_INVALID
MODEL_BACKEND_HTTP_ERROR
PADDLEOCR_SERVICE_UNAVAILABLE
SURYA_SERVICE_UNAVAILABLE
DOCLING_SERVICE_UNAVAILABLE
```

---

# Этап 12. Обновить processing trace

Добавлять stages:

```text
model_backend_health_check
paddleocr_http_ocr
surya_http_layout
surya_http_table
docling_http_parse
model_backend_fallback
```

Пример:

```json
{
  "name": "paddleocr_http_ocr",
  "status": "ok",
  "tool": "paddleocr_ppocrv6_medium_http",
  "duration_ms": 120,
  "metadata": {
    "backend_url": "http://127.0.0.1:8101",
    "regions": 15,
    "fallback_used": false
  }
}
```

---

# Этап 13. Обновить `model_outputs`

Записывать:

```jsonc
{
  "model_outputs": {
    "ocr": {
      "backend": "paddleocr_ppocrv6_medium",
      "backend_type": "http",
      "url": "http://127.0.0.1:8101",
      "fallback_used": false,
      "fallback_backend": null,
      "detection_model": "PaddlePaddle/PP-OCRv6_medium_det",
      "recognition_model": "PaddlePaddle/PP-OCRv6_medium_rec",
      "languages": ["ru", "en"],
      "pages_processed": 1,
      "elements_created": 12
    },
    "layout": {
      "backend": "surya_layout",
      "backend_type": "http",
      "url": "http://127.0.0.1:8102",
      "fallback_used": false,
      "regions_detected": 8
    },
    "structured_document_parse": {
      "backend": "docling",
      "backend_type": "http",
      "url": "http://127.0.0.1:8103",
      "executed": true,
      "fallback_used": false
    }
  }
}
```

---

# Этап 14. Документация

Создать:

```text
docs/MODEL_SERVICES_DEV.md
docs/REAL_MODEL_RUNTIME.md
```

## `MODEL_SERVICES_DEV.md`

Описать:

```text
- как поднять model services в mock mode;
- как проверить health;
- как переключить service в real mode;
- какие ports:
  - 8101 PaddleOCR
  - 8102 Surya
  - 8103 Docling
- как API использует эти services;
- что делать, если service недоступен.
```

## `REAL_MODEL_RUNTIME.md`

Описать:

```text
Phase 1:
  PP-OCRv6 + Surya + Docling

Phase 2:
  GLiNER + USER-bge-m3/BGE-M3

Phase 3:
  PaddleOCR-VL-1.6 + Qwen3-VL + Granite Docling

Phase 4:
  pix2tex/Pix2Text

Phase 5:
  Kraken/eScriptorium/Calamari
```

---

# Этап 15. Tests

Добавить:

```text
tests/http_model_backend_tests.rs
tests/paddleocr_http_adapter_tests.rs
tests/surya_http_adapter_tests.rs
tests/docling_http_adapter_tests.rs
tests/model_service_fallback_tests.rs
tests/doctor_model_services_tests.rs
```

## http_model_backend_tests

Проверить:

```text
health_check handles unavailable service
post_json handles timeout
post_json handles invalid response
```

## adapter tests

Использовать mock HTTP server или fake client.

Проверить:

```text
PaddleOCR response -> text_ocr elements
Surya layout response -> layout regions
Docling parse response -> structured output
```

## fallback tests

Проверить:

```text
PaddleOCR unavailable -> mock fallback
Surya unavailable -> heuristic fallback
Docling unavailable -> native/mock fallback
```

## doctor tests

Проверить:

```text
doctor reports service unavailable as WARN if required=false
doctor reports ERROR if required=true
```

---

# Этап 16. Smoke scripts

Создать:

```text
scripts/dev_start_model_services.sh
scripts/dev_check_model_services.sh
scripts/dev_smoke_real_runtime_phase1.sh
```

## `dev_check_model_services.sh`

Проверяет:

```bash
curl http://127.0.0.1:8101/healthz
curl http://127.0.0.1:8102/healthz
curl http://127.0.0.1:8103/healthz
```

## `dev_smoke_real_runtime_phase1.sh`

Проверяет:

```text
1. model services health
2. parser API health
3. upload image/pdf
4. job completed/partial
5. model.json contains model_outputs.ocr/layout/structured_document_parse
```

---

# Этап 17. Acceptance criteria

Задача считается выполненной, если:

1. Добавлен HTTP model backend client в Rust.
2. Добавлены HTTP schemas.
3. Добавлен PaddleOCR PP-OCRv6 HTTP adapter.
4. Добавлен Surya OCR/layout/table HTTP adapter.
5. Добавлен Docling structured/layout/table HTTP adapter.
6. Добавлены Python service skeletons:
   - `model_services/paddleocr_service`
   - `model_services/surya_service`
   - `model_services/docling_service`
7. Все services поддерживают mock mode.
8. Добавлен `docker-compose.models.dev.yml`.
9. Добавлен full dev compose или документация, как запускать API + services.
10. `doctor` проверяет model services.
11. Если service недоступен — pipeline делает fallback, а не падает.
12. `model_outputs` отражает backend, url, fallback_used.
13. Processing trace содержит model backend stages.
14. Документация добавлена.
15. Tests добавлены.
16. Existing tests не сломаны.
17. Проект компилируется:

```bash
cargo check
```

18. Tests проходят:

```bash
cargo test
```

19. Mock services поднимаются:

```bash
docker compose -f docker-compose.models.dev.yml up --build
```

20. Health работает:

```bash
curl http://127.0.0.1:8101/healthz
curl http://127.0.0.1:8102/healthz
curl http://127.0.0.1:8103/healthz
```

---

# 18. Не делать в этом PR

Не нужно:

```text
поднимать реальные PaddleOCR weights
поднимать реальные Surya models
поднимать реальные Docling pipeline
подключать PaddleOCR-VL-1.6
подключать Qwen3-VL
подключать Granite Docling
подключать GLiNER
подключать BGE-M3/USER-bge-m3
подключать pix2tex/Pix2Text
подключать Kraken/eScriptorium/Calamari
GPU/TensorRT/Triton
Kubernetes
production auth
```

Этот PR — Phase 1 runtime foundation.

---

# 19. После этого PR

Следующий этап:

```text
Real Model Runtime Phase 2:
  - включить реальные PP-OCRv6 weights/service
  - включить real Surya service
  - включить real Docling service
  - benchmark на dev corpus
  - quality report
```