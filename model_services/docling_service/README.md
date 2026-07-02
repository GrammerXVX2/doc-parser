# Docling Service (Dev)

## Run locally

```bash
uvicorn model_services.docling_service.app:app --host 0.0.0.0 --port 8103
```

## Environment

- `DOCLING_BACKEND=mock|real`
- `DOCLING_DEVICE=cpu|gpu`

## Endpoints

- `GET /healthz`
- `POST /v1/parse`
- `POST /v1/layout`
- `POST /v1/tables`

Mock mode is deterministic and enabled by default.
