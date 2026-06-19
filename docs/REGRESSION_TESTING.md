# Regression Testing

## Как добавить кейс

1. Создать директорию `regression/corpus/<format>/<case_name>/`.
2. Добавить `input.<ext>`.
3. Добавить `case.jsonc` с assertions/tolerances.
4. (Опционально) добавить fixtures.

## Обновление golden snapshots

```bash
scripts/run_regression.sh --update-golden
```

## Запуск regression

```bash
scripts/run_regression.sh
```

Или напрямую:

```bash
cargo test --test regression_tests
```

## Нормализуемые поля

- document_id
- job_id
- uploaded_at/processed_at
- duration_ms и *_ms
- sha256
- asset_id
- path
- hostname
