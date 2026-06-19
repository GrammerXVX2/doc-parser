# Benchmark Suite

Наборы данных:

- benchmarks/datasets/small_ru
- benchmarks/datasets/mixed_formats
- benchmarks/datasets/ocr_heavy
- benchmarks/datasets/office_heavy
- benchmarks/datasets/pdf_heavy

Отчеты сохраняются в:

- benchmarks/reports

## Запуск

```bash
scripts/run_benchmarks.sh
```

Скрипт выполняет benchmark в режимах:

- mock
- cpu
- gpu
- triton

Каждый запуск формирует JSON/Markdown отчет.
