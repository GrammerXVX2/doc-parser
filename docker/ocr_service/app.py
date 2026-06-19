from __future__ import annotations

from pathlib import Path
from typing import List, Optional

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

try:
    from paddleocr import PaddleOCR
except Exception:  # pragma: no cover
    PaddleOCR = None


class OcrRequest(BaseModel):
    image_path: str
    languages: List[str] = ["en"]


class OcrLine(BaseModel):
    text: str
    confidence: Optional[float] = None
    bbox: Optional[List[float]] = None


class OcrResponse(BaseModel):
    text: str
    lines: List[OcrLine]


app = FastAPI(title="document-parser-ocr", version="1.0.0")

_ocr = None


@app.on_event("startup")
def startup() -> None:
    global _ocr
    if PaddleOCR is None:
        return
    _ocr = PaddleOCR(use_angle_cls=True, lang="en", use_gpu=True)


@app.get("/health")
def health() -> dict:
    return {"ok": True, "engine": "paddleocr" if _ocr is not None else "unavailable"}


@app.post("/ocr", response_model=OcrResponse)
def ocr(req: OcrRequest) -> OcrResponse:
    if _ocr is None:
        raise HTTPException(status_code=503, detail="PaddleOCR is not available")

    image_path = Path(req.image_path)
    if not image_path.exists():
        raise HTTPException(status_code=404, detail=f"Image path not found: {req.image_path}")

    result = _ocr.ocr(str(image_path), cls=True)

    lines: List[OcrLine] = []
    full_text_parts: List[str] = []

    if not result:
        return OcrResponse(text="", lines=[])

    for block in result:
        if not block:
            continue
        for line in block:
            points = line[0]
            content = line[1][0] if line[1] else ""
            conf = float(line[1][1]) if line[1] and len(line[1]) > 1 else None
            if not content:
                continue

            xs = [p[0] for p in points]
            ys = [p[1] for p in points]
            bbox = [min(xs), min(ys), max(xs), max(ys)]

            lines.append(OcrLine(text=content, confidence=conf, bbox=bbox))
            full_text_parts.append(content)

    return OcrResponse(text="\n".join(full_text_parts), lines=lines)
