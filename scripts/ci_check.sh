#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[ci] cargo fmt --check"
cargo fmt --check

echo "[ci] cargo clippy --all-targets"
cargo clippy --all-targets -- -D warnings

echo "[ci] cargo test"
cargo test

echo "[ci] smoke test"
scripts/smoke_test.sh

echo "[ci] done"
