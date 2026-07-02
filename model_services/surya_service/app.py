import os
from fastapi import FastAPI
from pydantic import BaseModel, Field


app = FastAPI(title="Surya Service", version="0.1.0")


class BaseRequest(BaseModel):
    document_id: str
    page_number: int
    image_path: str | None = None
    width: float | None = None
    height: float | None = None
    languages: list[str] = Field(default_factory=lambda: ["ru", "en"])
    options: dict = Field(default_factory=dict)


@app.get("/healthz")
def healthz() -> dict:
    return {
        "status": "ok",
        "service": "surya_service",
        "mode": os.getenv("SURYA_BACKEND", "mock"),
    }


@app.post("/v1/ocr")
def ocr(payload: BaseRequest) -> dict:
    mode = os.getenv("SURYA_BACKEND", "mock")
    return {
        "backend": "surya_ocr",
        "regions": [
            {
                "text": f"Surya OCR mock {payload.document_id} p{payload.page_number}",
                "bbox": [60, 90, 700, 130],
                "confidence": 0.91,
                "language": payload.languages[0] if payload.languages else "ru",
            }
        ],
        "text": f"Surya OCR mock {payload.document_id} p{payload.page_number}",
        "confidence": 0.91,
        "metadata": {"mode": mode},
    }


@app.post("/v1/layout")
def layout(payload: BaseRequest) -> dict:
    mode = os.getenv("SURYA_BACKEND", "mock")
    return {
        "backend": "surya_layout",
        "regions": [
            {
                "region_type": "title",
                "bbox": [50, 40, 900, 100],
                "confidence": 0.95,
                "text": None,
            },
            {
                "region_type": "text",
                "bbox": [50, 120, 900, 500],
                "confidence": 0.90,
                "text": None,
            },
        ],
        "metadata": {"mode": mode},
    }


@app.post("/v1/tables")
def tables(payload: BaseRequest) -> dict:
    mode = os.getenv("SURYA_BACKEND", "mock")
    return {
        "backend": "surya_table",
        "regions": [
            {
                "region_type": "table",
                "bbox": [100, 300, 900, 700],
                "confidence": 0.88,
                "text": None,
            }
        ],
        "metadata": {"mode": mode},
    }
