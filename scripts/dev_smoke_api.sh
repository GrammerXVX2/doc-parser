#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${1:-http://127.0.0.1:8080}"
INPUT_FILE="${2:-testdata/ru/sample_ru.html}"

if [[ ! -f "$INPUT_FILE" ]]; then
  echo "SMOKE_ERROR: input file not found: $INPUT_FILE" >&2
  exit 1
fi

parse_json_field() {
  local json="$1"
  local expr="$2"
  python3 - "$expr" "$json" <<'PY'
import json
import sys

expr = sys.argv[1]
try:
    payload = json.loads(sys.argv[2])
except Exception:
    print("")
    sys.exit(0)
node = payload
for part in expr.split('.'):
    if isinstance(node, dict):
        node = node.get(part)
    else:
        node = None
        break
if node is None:
    print("")
elif isinstance(node, (dict, list)):
    print(json.dumps(node, ensure_ascii=False))
else:
    print(str(node))
PY
}

echo "[smoke] healthz"
curl -fsS "$BASE_URL/healthz" >/dev/null

echo "[smoke] readyz"
curl -fsS "$BASE_URL/readyz" >/dev/null

echo "[smoke] upload"
UPLOAD_RESPONSE="$(curl -fsS \
  -F "file=@${INPUT_FILE}" \
  -F "language=ru" \
  -F "extract_tables=true" \
  -F "table_chunks=true" \
  "$BASE_URL/v1/documents")"

JOB_ID="$(parse_json_field "$UPLOAD_RESPONSE" "job_id")"
DOCUMENT_ID="$(parse_json_field "$UPLOAD_RESPONSE" "document_id")"

if [[ -z "$JOB_ID" || -z "$DOCUMENT_ID" ]]; then
  echo "SMOKE_ERROR: failed to parse job_id/document_id" >&2
  echo "$UPLOAD_RESPONSE" >&2
  exit 1
fi

echo "[smoke] job_id=$JOB_ID document_id=$DOCUMENT_ID"

STATUS=""
for _ in $(seq 1 60); do
  JOB_RESPONSE="$(curl -fsS "$BASE_URL/v1/jobs/$JOB_ID")"
  STATUS="$(parse_json_field "$JOB_RESPONSE" "status")"

  case "$STATUS" in
    completed|partial|failed)
      break
      ;;
    *)
      sleep 1
      ;;
  esac
done

if [[ "$STATUS" == "failed" ]]; then
  echo "SMOKE_ERROR: job failed" >&2
  curl -fsS "$BASE_URL/v1/jobs/$JOB_ID" >&2 || true
  exit 1
fi

if [[ "$STATUS" != "completed" && "$STATUS" != "partial" ]]; then
  echo "SMOKE_ERROR: job status timeout, last status='$STATUS'" >&2
  exit 1
fi

echo "[smoke] verify outputs"
curl -fsS "$BASE_URL/v1/documents/$DOCUMENT_ID/model" >/dev/null
curl -fsS "$BASE_URL/v1/documents/$DOCUMENT_ID/markdown" >/dev/null
curl -fsS "$BASE_URL/v1/documents/$DOCUMENT_ID/text" >/dev/null

echo "[smoke] success"
