# Regression Corpus

Регрессионный набор хранится по форматам и кейсам:

- regression/corpus/<format>/<case_name>/input.<ext>
- regression/corpus/<format>/<case_name>/case.jsonc
- regression/corpus/<format>/<case_name>/fixtures/

Golden snapshots хранятся в:

- regression/expected/<format>/<case_id>.model.json

## Запуск

Полный прогон:

```bash
scripts/run_regression.sh
```

Обновление golden snapshots:

```bash
scripts/run_regression.sh --update-golden
```

Отдельная проверка структуры и наличия golden snapshots:

```bash
python3 scripts/validate_golden.py --corpus regression/corpus --expected regression/expected
```

## Нормализация

Перед сравнением snapshots нормализуются нестабильные поля:

- document_id
- job_id
- uploaded_at
- processed_at
- duration_ms и поля *_ms
- sha256
- asset_id
- path
- hostname
