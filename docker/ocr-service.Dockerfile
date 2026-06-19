FROM python:3.11-slim

WORKDIR /workspace

RUN apt-get update && apt-get install -y --no-install-recommends \
    libglib2.0-0 \
    libsm6 \
    libxrender1 \
    libxext6 \
    && rm -rf /var/lib/apt/lists/*

RUN pip install --no-cache-dir \
    fastapi==0.116.1 \
    uvicorn[standard]==0.35.0 \
    pydantic==2.11.9 \
    paddlepaddle-gpu==2.6.1 \
    paddleocr==2.9.1 \
    pillow==11.3.0

COPY docker/ocr_service ./docker/ocr_service

CMD ["uvicorn", "docker.ocr_service.app:app", "--host", "0.0.0.0", "--port", "8000"]
