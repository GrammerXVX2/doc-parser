#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

mkdir -p benchmarks/reports

declare -A PROFILE_BY_MODE
PROFILE_BY_MODE[mock]="configs/profiles/benchmark.jsonc"
PROFILE_BY_MODE[cpu]="configs/profiles/benchmark.jsonc"
PROFILE_BY_MODE[gpu]="configs/profiles/gpu.jsonc"
PROFILE_BY_MODE[triton]="configs/profiles/triton.jsonc"

for mode in mock cpu gpu triton; do
  echo "[benchmark] mode=$mode dataset=small_ru"
  out="benchmarks/reports/${mode}_small_ru"
  mkdir -p "$out"
  cargo run -- bench \
    --input benchmarks/datasets/small_ru \
    --output "$out" \
    --profile "${PROFILE_BY_MODE[$mode]}" || true

done

echo "[benchmark] отчеты сохранены в benchmarks/reports"
