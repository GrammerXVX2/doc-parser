#!/usr/bin/env bash
set -euo pipefail

rm -rf data/input/* data/output/* data/metadata/jobs/*
mkdir -p data/input data/output data/metadata/jobs
touch data/input/.gitkeep data/output/.gitkeep data/metadata/jobs/.gitkeep

echo "Dev data cleaned."
