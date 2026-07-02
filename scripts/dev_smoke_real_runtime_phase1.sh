#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

# 1) model services health
./scripts/dev_check_model_services.sh

# 2) parser API health
curl -fsS http://127.0.0.1:8080/healthz | cat
curl -fsS http://127.0.0.1:8080/readyz | cat

# 3) upload sample file
BOUNDARY="x-boundary-real-runtime"
PAYLOAD_FILE="/tmp/doc_parser_smoke_payload.bin"

{
  printf -- '--%s\r\n' "$BOUNDARY"
  printf 'Content-Disposition: form-data; name="file"; filename="sample_ru.html"\r\n'
  printf 'Content-Type: text/html\r\n\r\n'
  cat testdata/ru/sample_ru.html
  printf '\r\n--%s\r\n' "$BOUNDARY"
  printf 'Content-Disposition: form-data; name="model_profile"\r\n\r\nmixed_enterprise'
  printf '\r\n--%s\r\n' "$BOUNDARY"
  printf 'Content-Disposition: form-data; name="enable_slow_path"\r\n\r\ntrue'
  printf '\r\n--%s--\r\n' "$BOUNDARY"
} > "$PAYLOAD_FILE"

UPLOAD_JSON="$(curl -fsS -X POST http://127.0.0.1:8080/v1/documents \
  -H "Content-Type: multipart/form-data; boundary=$BOUNDARY" \
  --data-binary "@$PAYLOAD_FILE")"

echo "$UPLOAD_JSON" | cat

JOB_ID="$(echo "$UPLOAD_JSON" | sed -n 's/.*"job_id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')"
DOC_ID="$(echo "$UPLOAD_JSON" | sed -n 's/.*"document_id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')"

if [[ -z "$JOB_ID" || -z "$DOC_ID" ]]; then
  echo "Failed to parse job_id/document_id from upload response"
  exit 1
fi

# 4) wait for completed/partial
for _ in $(seq 1 60); do
  STATUS_JSON="$(curl -fsS "http://127.0.0.1:8080/v1/jobs/$JOB_ID")"
  STATUS="$(echo "$STATUS_JSON" | sed -n 's/.*"status"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')"
  if [[ "$STATUS" == "completed" || "$STATUS" == "partial" ]]; then
    break
  fi
  sleep 1
done

if [[ "$STATUS" != "completed" && "$STATUS" != "partial" ]]; then
  echo "Job did not reach completed/partial status"
  exit 1
fi

# 5) check model outputs
MODEL_JSON="$(curl -fsS "http://127.0.0.1:8080/v1/documents/$DOC_ID/model")"
echo "$MODEL_JSON" | grep -q '"model_outputs"'
echo "$MODEL_JSON" | grep -q '"ocr"'
echo "$MODEL_JSON" | grep -q '"layout"'
echo "$MODEL_JSON" | grep -q '"structured_document_parse"'

echo "Phase 1 real runtime smoke passed"
