#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

UPDATE_GOLDEN=false
if [[ "${1:-}" == "--update-golden" ]]; then
  UPDATE_GOLDEN=true
fi

echo "[regression] запуск regression suite"

cargo test --test regression_tests

if [[ "$UPDATE_GOLDEN" == "true" ]]; then
  echo "[regression] обновление golden snapshots"
  rm -rf target/regression_update
  mkdir -p target/regression_update

  while IFS= read -r case; do
    case_dir="$(dirname "$case")"
    format="$(basename "$(dirname "$case_dir")")"
    case_id="$(sed -n 's/.*"case_id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$case" | head -n 1)"
    input_file="$(find "$case_dir" -maxdepth 1 -type f ! -name 'case.jsonc' ! -name 'README.md' | head -n 1)"

    if [[ -z "$case_id" || -z "$input_file" ]]; then
      echo "[regression] skip: case=$case"
      continue
    fi

    out_dir="target/regression_update/${format}/${case_id}"
    mkdir -p "$out_dir"

    cargo run -- parse "$input_file" --output "$out_dir" >/tmp/reg_update_${case_id}.log 2>&1
    model_path="$(find "$out_dir" -name model.json | head -n 1)"
    if [[ -z "$model_path" ]]; then
      echo "[regression] ошибка: model.json не найден для ${format}/${case_id}"
      tail -n 20 "/tmp/reg_update_${case_id}.log" || true
      exit 1
    fi

    mkdir -p "regression/expected/${format}"
    cp "$model_path" "regression/expected/${format}/${case_id}.model.json"
    echo "[regression] golden: ${format}/${case_id}"
  done < <(find regression/corpus -name case.jsonc | sort)
fi

python3 scripts/validate_golden.py --corpus regression/corpus --expected regression/expected

echo "[regression] успешно"
