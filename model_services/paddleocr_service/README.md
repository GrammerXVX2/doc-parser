# PaddleOCR Service (Dev)

## Run locally

```bash
uvicorn model_services.paddleocr_service.app:app --host 0.0.0.0 --port 8101
```

## Environment

- `PADDLEOCR_BACKEND=mock|real`
- `PADDLEOCR_DEVICE=cpu|gpu`
- `PADDLEOCR_LANG=ru,en`
- `PADDLEOCR_DET_MODEL=PaddlePaddle/PP-OCRv6_medium_det`
- `PADDLEOCR_REC_MODEL=PaddlePaddle/PP-OCRv6_medium_rec`

## Endpoints

- `GET /healthz`
- `POST /v1/ocr`

Mock mode is deterministic and enabled by default.
