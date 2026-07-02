#!/usr/bin/env bash
set -euo pipefail

curl -fsS http://127.0.0.1:8101/healthz | cat
curl -fsS http://127.0.0.1:8102/healthz | cat
curl -fsS http://127.0.0.1:8103/healthz | cat

echo "All model services health checks passed"
