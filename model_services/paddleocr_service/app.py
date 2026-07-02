import os
from fastapi import FastAPI
from pydantic import BaseModel, Field


app = FastAPI(title="PaddleOCR Service", version="0.1.0")


class OcrRequest(BaseModel):
    document_id: str
    page_number: int
    image_path: str
    languages: list[str] = Field(default_factory=lambda: ["ru", "en"])
    options: dict = Field(default_factory=dict)


@app.get("/healthz")
def healthz() -> dict:
    return {
        "status": "ok",
        "service": "paddleocr_service",
        "mode": os.getenv("PADDLEOCR_BACKEND", "mock"),
    }


@app.post("/v1/ocr")
def ocr(payload: OcrRequest) -> dict:
    mode = os.getenv("PADDLEOCR_BACKEND", "mock")

    return {
        "backend": "paddleocr_ppocrv6_medium",
        "regions": [
            {
                "text": f"Mock OCR text for {payload.document_id} page {payload.page_number}",
                "bbox": [100, 100, 500, 140],
                "confidence": 0.95,
                "language": payload.languages[0] if payload.languages else "ru",
            }
        ],
        "text": f"Mock OCR text for {payload.document_id} page {payload.page_number}",
        "confidence": 0.95,
        "metadata": {
            "mode": mode,
            "image_path": payload.image_path,
        },
    }
