#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[smoke] cargo check"
cargo check

echo "[smoke] cargo test"
cargo test

echo "[smoke] parse sample html"
rm -rf target/smoke_output
mkdir -p target/smoke_output
cargo run -- parse testdata/ru/sample_ru.html --output target/smoke_output

MODEL_PATH="$(find target/smoke_output -name model.json | head -n 1)"
if [[ -z "$MODEL_PATH" ]]; then
  echo "SMOKE_TEST_FAILED: model.json не найден"
  exit 1
fi

echo "[smoke] quality report"
cargo run -- quality --input "$MODEL_PATH"

echo "[smoke] done"
