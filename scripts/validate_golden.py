#!/usr/bin/env python3
"""Проверка структуры и наличия golden snapshots для regression corpus."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


REQUIRED_KEYS = [
    "schema_version",
    "source",
    "document_profile",
    "pages",
]


def strip_jsonc(text: str) -> str:
    # Remove // and /* */ comments to parse basic JSONC without external deps.
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.DOTALL)
    text = re.sub(r"(^|\s)//.*$", "", text, flags=re.MULTILINE)
    return text


def parse_case_id(case_path: Path) -> str:
    raw = case_path.read_text(encoding="utf-8")
    cleaned = strip_jsonc(raw)
    try:
        payload = json.loads(cleaned)
    except json.JSONDecodeError as exc:
        raise ValueError(f"Некорректный JSONC в {case_path}: {exc}") from exc

    case_id = payload.get("case_id")
    if not isinstance(case_id, str) or not case_id.strip():
        raise ValueError(f"В {case_path} отсутствует обязательное поле case_id")
    return case_id.strip()


def validate_snapshot(path: Path) -> list[str]:
    errors: list[str] = []
    if not path.exists():
        return [f"Отсутствует golden snapshot: {path}"]

    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        return [f"Некорректный JSON в {path}: {exc}"]

    for key in REQUIRED_KEYS:
        if key not in data:
            errors.append(f"В {path} отсутствует поле '{key}'")

    pages = data.get("pages")
    if not isinstance(pages, list):
        errors.append(f"В {path} поле 'pages' должно быть массивом")

    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description="Валидация regression golden snapshots")
    parser.add_argument("--corpus", default="regression/corpus", help="Путь до regression corpus")
    parser.add_argument("--expected", default="regression/expected", help="Путь до golden snapshots")
    args = parser.parse_args()

    corpus_root = Path(args.corpus)
    expected_root = Path(args.expected)

    if not corpus_root.exists():
        print(f"Ошибка: corpus каталог не найден: {corpus_root}")
        return 1
    if not expected_root.exists():
        print(f"Ошибка: expected каталог не найден: {expected_root}")
        return 1

    case_files = sorted(corpus_root.rglob("case.jsonc"))
    if not case_files:
        print("Ошибка: не найдено ни одного case.jsonc")
        return 1

    problems: list[str] = []
    validated = 0

    for case_file in case_files:
        fmt = case_file.parent.parent.name
        try:
            case_id = parse_case_id(case_file)
        except ValueError as exc:
            problems.append(str(exc))
            continue

        snapshot = expected_root / fmt / f"{case_id}.model.json"
        errors = validate_snapshot(snapshot)
        if errors:
            problems.extend(errors)
        else:
            validated += 1

    if problems:
        print("Проверка golden snapshots завершилась с ошибками:")
        for item in problems:
            print(f"- {item}")
        return 1

    print(f"Проверка golden snapshots успешна: валидировано кейсов {validated}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
