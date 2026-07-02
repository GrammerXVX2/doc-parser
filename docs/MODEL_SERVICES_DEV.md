# MODEL SERVICES DEV

Phase 1 model services:

- PaddleOCR service: `http://127.0.0.1:8101`
- Surya service: `http://127.0.0.1:8102`
- Docling service: `http://127.0.0.1:8103`

## Start model services (mock mode)

```bash
docker compose -f docker-compose.models.dev.yml up --build -d
```

или

```bash
./scripts/dev_start_model_services.sh
```

## Health checks

```bash
curl http://127.0.0.1:8101/healthz
curl http://127.0.0.1:8102/healthz
curl http://127.0.0.1:8103/healthz
```

или

```bash
./scripts/dev_check_model_services.sh
```

## Switch to real mode

Для каждого сервиса установите backend переменную:

- `PADDLEOCR_BACKEND=real`
- `SURYA_BACKEND=real`
- `DOCLING_BACKEND=real`

Также можно указать device (`cpu|gpu`) и model-specific env vars.

## API integration

Parser API читает `configs/model_stack.config.jsonc` и использует HTTP endpoints:

- PaddleOCR `/v1/ocr`
- Surya `/v1/ocr`, `/v1/layout`, `/v1/tables`
- Docling `/v1/parse`, `/v1/layout`, `/v1/tables`

Используйте `docker-compose.full.dev.yml`, чтобы поднять API + model services вместе.

## Failure behavior

Если service недоступен:

- записывается structured warning;
- активируется fallback (mock/heuristic/placeholder);
- pipeline продолжается и `model.json` создается.
