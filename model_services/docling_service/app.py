import os
from fastapi import FastAPI
from pydantic import BaseModel, Field


app = FastAPI(title="Docling Service", version="0.1.0")


class ParseRequest(BaseModel):
    document_id: str
    input_path: str
    format: str = "pdf"
    language: str = "ru"
    options: dict = Field(default_factory=dict)


class LayoutRequest(BaseModel):
    document_id: str
    page_number: int
    image_path: str | None = None
    width: float | None = None
    height: float | None = None
    options: dict = Field(default_factory=dict)


@app.get("/healthz")
def healthz() -> dict:
    return {
        "status": "ok",
        "service": "docling_service",
        "mode": os.getenv("DOCLING_BACKEND", "mock"),
    }


@app.post("/v1/parse")
def parse(payload: ParseRequest) -> dict:
    mode = os.getenv("DOCLING_BACKEND", "mock")
    return {
        "backend": "docling",
        "markdown": "# Документ\\n\\nТекст документа.",
        "text": "Документ\\n\\nТекст документа.",
        "elements": [],
        "tables": [],
        "metadata": {
            "mode": mode,
            "input_path": payload.input_path,
            "format": payload.format,
        },
        "confidence": 0.9,
    }


@app.post("/v1/layout")
def layout(payload: LayoutRequest) -> dict:
    mode = os.getenv("DOCLING_BACKEND", "mock")
    return {
        "backend": "docling_layout",
        "regions": [
            {
                "region_type": "title",
                "bbox": [60, 40, 920, 110],
                "confidence": 0.94,
                "text": None,
            },
            {
                "region_type": "text",
                "bbox": [60, 120, 920, 820],
                "confidence": 0.9,
                "text": None,
            },
        ],
        "metadata": {"mode": mode},
    }


@app.post("/v1/tables")
def tables(payload: LayoutRequest) -> dict:
    mode = os.getenv("DOCLING_BACKEND", "mock")
    return {
        "backend": "docling_tableformer",
        "regions": [
            {
                "region_type": "table",
                "bbox": [120, 280, 920, 740],
                "confidence": 0.89,
                "text": None,
            }
        ],
        "metadata": {"mode": mode},
    }
