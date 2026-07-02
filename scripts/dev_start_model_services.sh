#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

docker compose -f docker-compose.models.dev.yml up --build -d

echo "Model services started (mock mode):"
echo "- PaddleOCR: http://127.0.0.1:8101/healthz"
echo "- Surya:     http://127.0.0.1:8102/healthz"
echo "- Docling:   http://127.0.0.1:8103/healthz"
