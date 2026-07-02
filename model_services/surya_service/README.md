# Surya Service (Dev)

## Run locally

```bash
uvicorn model_services.surya_service.app:app --host 0.0.0.0 --port 8102
```

## Environment

- `SURYA_BACKEND=mock|real`
- `SURYA_DEVICE=cpu|gpu`

## Endpoints

- `GET /healthz`
- `POST /v1/ocr`
- `POST /v1/layout`
- `POST /v1/tables`

Mock mode is deterministic and enabled by default.
