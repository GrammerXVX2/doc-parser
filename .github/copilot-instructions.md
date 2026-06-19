# Задача: Stage 10 — Production hardening, regression suite, benchmarks, deployment docs

## Контекст

Проект уже должен иметь Stage 1–9.

Система уже имеет:

- canonical `DocumentModel`;
- CLI parser;
- HTTP API;
- job queue;
- local storage;
- observability;
- security limits;
- HTML/Markdown/TXT/PDF/Image/DOCX/XLSX/PPTX/RTF/DOC support;
- OCR interfaces;
- ONNX OCR backend architecture;
- mock/fixture/real OCR hooks;
- layout/table/formula abstractions;
- Russian-first language defaults;
- Office extractors;
- converter sandbox;
- performance configs;
- dynamic batching foundation;
- model registry;
- warmup;
- benchmark command.

Теперь нужен финальный production hardening слой.

---

# Главная цель Stage 10

Реализовать:

```text
1. Regression corpus structure.
2. Golden snapshot tests for model.json.
3. Quality metrics for extraction.
4. End-to-end benchmark suite.
5. Failure injection tests.
6. Security hardening checklist and enforcement tests.
7. Dockerfile and deployment docs.
8. Operational runbooks.
9. Final documentation index.
10. Release readiness checks.
```

---

# Фундаментальное требование

## Русский язык по умолчанию

Все пользовательские сообщения, документация для оператора и примеры должны быть на русском.

Machine-readable codes остаются английскими uppercase.

---

# Не делать

Не нужно реализовывать:

```text
cloud-specific Kubernetes manifests
real S3 production deployment
real auth/billing/multitenancy
real custom CUDA kernels
full INT8 calibration pipeline
```

Но документация должна указать, куда это добавлять.

---

# Новые/доработанные директории

Добавить:

```text
regression/
  corpus/
    html/
    markdown/
    txt/
    pdf/
    images/
    docx/
    xlsx/
    pptx/
    legacy/
  expected/
    html/
    markdown/
    txt/
    pdf/
    images/
    docx/
    xlsx/
    pptx/
    legacy/
  README.md

benchmarks/
  datasets/
  reports/
  README.md

docs/
  PRODUCTION.md
  DEPLOYMENT.md
  SECURITY.md
  OBSERVABILITY.md
  RUNBOOK.md
  QUALITY_METRICS.md
  REGRESSION_TESTING.md
  RELEASE_CHECKLIST.md

scripts/
  run_regression.sh
  run_benchmarks.sh
  validate_golden.py или .rs
  smoke_test.sh

docker/
  Dockerfile
  docker-compose.local.yml
  README.md
```

If Python scripts are not desired, use Rust binaries instead.  
Prefer Rust where practical, shell for orchestration is acceptable.

---

# Этап 1: Regression corpus layout

Create documented corpus structure.

Each test case:

```text
regression/corpus/<format>/<case_name>/
  input.<ext>
  case.jsonc
  fixtures/
  README.md optional
```

Example `case.jsonc`:

```jsonc
{
  "case_id": "ru_html_basic",
  "format": "html",
  "language": "ru",
  "description": "Базовый HTML-документ на русском языке.",
  "expected": {
    "min_pages": 1,
    "min_elements": 3,
    "must_contain_text": [
      "Пример документа",
      "Таблица"
    ],
    "must_have_element_types": [
      "heading",
      "text"
    ],
    "must_have_chunks": true
  },
  "tolerances": {
    "ignore_fields": [
      "document_id",
      "job_id",
      "processed_at",
      "duration_ms",
      "sha256",
      "asset_id",
      "path"
    ]
  }
}
```

---

# Этап 2: Golden snapshot framework

Implement snapshot tests.

Goal:

```text
Run parser on corpus input
normalize nondeterministic fields
compare with expected model snapshot
```

Add module:

```text
src/regression/
  mod.rs
  normalize.rs
  runner.rs
  assertions.rs
```

Or integration test:

```text
tests/regression_tests.rs
```

## Normalization

Ignore/normalize:

```text
document_id
job_id
timestamps
duration_ms
absolute paths
sha256 if unstable
asset_id if generated randomly
processing runtime hostname
model warmup timings
```

Keep stable:

```text
schema_version
source extension/mime
document_profile
page_count
element types
content text
table structures
chunks metadata
warnings/error codes
```

---

# Этап 3: Assertion-based regression

Golden snapshots can be brittle. Add assertion-based checks.

Implement:

```rust
pub struct RegressionAssertions {
    pub min_pages: Option<usize>,
    pub min_elements: Option<usize>,
    pub must_contain_text: Vec<String>,
    pub must_have_element_types: Vec<String>,
    pub must_have_chunks: bool,
    pub must_have_tables: Option<bool>,
    pub must_have_ocr: Option<bool>,
    pub max_errors: Option<usize>,
}
```

Run both:

```text
1. structural assertions
2. optional golden snapshot diff
```

---

# Этап 4: Quality metrics

Implement extraction quality report.

```rust
pub struct QualityReport {
    pub document_id: String,
    pub format: String,
    pub language: String,
    pub pages: usize,
    pub elements: usize,
    pub chars: usize,
    pub words: usize,
    pub tables: usize,
    pub images: usize,
    pub formulas: usize,
    pub ocr_elements: usize,
    pub warnings: usize,
    pub errors: usize,
    pub empty_pages: usize,
    pub duplicate_text_ratio: f32,
    pub low_confidence_ocr_ratio: f32,
    pub chunk_count: usize,
}
```

Add command:

```bash
cargo run -- quality --input output/<document_id>/model.json
```

Output:

```text
quality_report.json
quality_report.md
```

---

# Этап 5: Duplicate text detection metric

Implement approximate duplicate text metric:

```text
- collect normalized text blocks
- count repeated identical/near-identical blocks
- duplicate_text_ratio = duplicate_chars / total_chars
```

This helps catch OCR/native duplication regression.

---

# Этап 6: Failure injection tests

Add tests for failure cases:

```text
missing input file
empty file
unsupported extension
corrupt zip/docx/xlsx/pptx
corrupt PDF
oversized file
image dimensions too large
external converter missing
OCR model missing
Triton unavailable
queue full
processing timeout
path traversal attempt
```

All should produce:

```text
structured error
no panic
Russian message
stable machine code
```

---

# Этап 7: Security hardening tests

Add tests for:

```text
path traversal in upload filename
path traversal in asset_id/path
OOXML zip slip
archive too many entries
archive uncompressed size limit
large image dimensions
external command no shell
external converter timeout
external converter stderr truncation
```

Create doc:

```text
docs/SECURITY.md
```

Include:

```text
- threat model
- supported file risks
- external converter risks
- sandbox policy
- current limits
- known limitations
- recommended production isolation
```

---

# Этап 8: End-to-end benchmark suite

Improve benchmark command from Stage 9.

Benchmark modes:

```text
mock
cpu
gpu
triton
```

Datasets:

```text
small_ru
mixed_formats
ocr_heavy
office_heavy
pdf_heavy
```

Report:

```json
{
  "profile": "benchmark",
  "dataset": "small_ru",
  "documents": 100,
  "pages": 250,
  "duration_ms": 10000,
  "documents_per_second": 10.0,
  "pages_per_second": 25.0,
  "latency_ms": {
    "p50": 100,
    "p95": 250,
    "p99": 500
  },
  "formats": {
    "pdf": 20,
    "docx": 10,
    "html": 30
  },
  "errors": {},
  "warnings": {},
  "ocr": {
    "pages": 50,
    "crops": 1000,
    "avg_recognition_batch_size": 64
  }
}
```

Script:

```bash
scripts/run_benchmarks.sh
```

---

# Этап 9: Smoke tests

Add:

```bash
scripts/smoke_test.sh
```

Should run:

```text
cargo check
cargo test
cargo run -- parse testdata/ru/sample_ru.html --output target/smoke_output
cargo run -- quality --input target/smoke_output/<doc>/model.json
```

If API feature enabled:

```text
start server
healthz
upload sample
poll job
fetch model
shutdown server
```

---

# Этап 10: Dockerfile

Create:

```text
docker/Dockerfile
```

Requirements:

```text
multi-stage build if practical
non-root user
working directory /app
config directory /app/configs
data directory /app/data
expose 8080
default command serve
```

Optional system packages:

```text
libreoffice
pandoc
poppler-utils
tesseract not required
```

Since external tools may be heavy, document two image variants:

```text
minimal
full-converters
```

MVP Dockerfile can be minimal and docs explain optional converter installation.

---

# Этап 11: Docker Compose local

Create:

```text
docker/docker-compose.local.yml
```

Service:

```text
document-parser
ports:
  - "8080:8080"
volumes:
  - ./data:/app/data
  - ./configs:/app/configs
environment:
  RUST_LOG: info
```

---

# Этап 12: Production docs

## docs/PRODUCTION.md

Include:

```text
- architecture overview
- runtime profiles
- recommended limits
- worker settings
- storage layout
- model loading
- OCR backend modes
```

## docs/DEPLOYMENT.md

Include:

```text
- local binary
- Docker
- systemd example
- environment variables
- config files
```

## docs/OBSERVABILITY.md

Include:

```text
- logs
- tracing fields
- metrics list
- Prometheus endpoint
- health/readiness
```

## docs/RUNBOOK.md

Include troubleshooting:

```text
OCR model not found
LibreOffice unavailable
queue full
high latency
too many partial documents
PDF render failures
OOM prevention
disk full
```

## docs/REGRESSION_TESTING.md

Include:

```text
how to add corpus case
how to update golden snapshots
how to run regression
what fields are normalized
```

## docs/QUALITY_METRICS.md

Include:

```text
quality report fields
duplicate ratio
low confidence OCR ratio
warnings/errors interpretation
```

## docs/RELEASE_CHECKLIST.md

Include:

```text
cargo check
cargo test
regression
benchmarks
smoke tests
Docker build
security tests
docs update
```

---

# Этап 13: Release readiness command

Add command:

```bash
cargo run -- doctor
```

Checks:

```text
config loads
output directory writable
input directory writable
optional converters availability
OCR model files if configured
Triton reachable if configured
Prometheus enabled if service profile
security limits sane
```

Output Russian report:

```text
Проверка окружения document-parser

OK: Конфигурация загружена
OK: Каталог вывода доступен
WARN: LibreOffice не найден
WARN: OCR model det.onnx не найден
```

JSON option:

```bash
cargo run -- doctor --json
```

---

# Этап 14: API readiness hardening

Ensure:

```text
readyz reports queue full
readyz reports storage unavailable
readyz reports model registry not warmed if required
```

Do not mark ready if fatal config invalid.

---

# Этап 15: Documentation index

Update root `README.md`:

```text
- What this project does
- Supported formats
- Quick start CLI
- Quick start API
- Output structure
- Russian-first behavior
- Links to docs
- Limitations
```

---

# Этап 16: CI-friendly checks

Even if no CI file is needed, create script:

```bash
scripts/ci_check.sh
```

Runs:

```text
cargo fmt --check
cargo clippy --all-targets -- -D warnings optional/configurable
cargo test
scripts/smoke_test.sh
```

If clippy too strict, document exceptions.

---

# Tests to add

## regression_tests.rs

```text
run corpus cases
assert structure
compare normalized snapshot if expected exists
```

## quality_tests.rs

```text
quality report generated
duplicate_text_ratio detects duplicates
low_confidence_ocr_ratio works
```

## failure_injection_tests.rs

```text
all failure cases return structured errors
no panic
Russian messages
```

## security_hardening_tests.rs

```text
path traversal blocked
zip slip blocked
limits enforced
external commands safe
```

## doctor_tests.rs

```text
doctor reports missing optional tools as warnings
doctor reports invalid required config as error
json output valid
```

---

# Error/warning codes

Add if missing:

```text
REGRESSION_ASSERTION_FAILED
GOLDEN_SNAPSHOT_MISMATCH
QUALITY_REPORT_FAILED
DUPLICATE_TEXT_HIGH
LOW_OCR_CONFIDENCE_RATIO_HIGH

DOCTOR_CONFIG_INVALID
DOCTOR_STORAGE_UNAVAILABLE
DOCTOR_MODEL_MISSING
DOCTOR_CONVERTER_MISSING
DOCTOR_TRITON_UNAVAILABLE

SECURITY_PATH_TRAVERSAL_BLOCKED
SECURITY_ZIP_SLIP_BLOCKED
SECURITY_LIMIT_EXCEEDED

SMOKE_TEST_FAILED
BENCHMARK_THRESHOLD_FAILED
```

Messages in Russian.

---

# Definition of Done

Задача считается выполненной, если:

1. Проект компилируется:

```bash
cargo check
```

2. Все тесты проходят:

```bash
cargo test
```

3. Regression corpus structure exists.

4. Regression runner/tests exist.

5. Golden snapshot normalization exists.

6. Assertion-based regression exists.

7. Quality report command exists:

```bash
cargo run -- quality --input <model.json>
```

8. Duplicate text ratio metric exists.

9. Failure injection tests exist.

10. Security hardening tests exist.

11. Benchmark suite/report is documented and runnable.

12. Smoke test script exists.

13. Dockerfile exists.

14. docker-compose.local.yml exists.

15. `doctor` command exists.

16. Production docs exist:

```text
PRODUCTION.md
DEPLOYMENT.md
SECURITY.md
OBSERVABILITY.md
RUNBOOK.md
REGRESSION_TESTING.md
QUALITY_METRICS.md
RELEASE_CHECKLIST.md
```

17. Root README links to documentation.

18. API readiness is hardened.

19. Russian locale/messages remain default.

20. Existing Stage 1–9 functionality is not broken.

21. No production `unwrap()`.

---

# После этого этапа

После Stage 10 проект считается production-skeleton complete.

Дальше работа идёт не промптами по архитектуре, а конкретными задачами:

```text
- подключить выбранные реальные OCR модели;
- подобрать layout/table/formula модели;
- провести benchmark на реальных документах;
- настроить GPU/TensorRT/Triton;
- добавить PostgreSQL/S3;
- добавить auth/multitenancy;
- добавить Kubernetes/Helm;
- улучшать качество на corpus regression failures.
```