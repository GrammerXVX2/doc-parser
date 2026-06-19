# Runbook

## OCR model not found

Симптомы: предупреждения DOCTOR_MODEL_MISSING, fallback_to_mock.

Действия:

1. Проверить пути model_path/charset_path.
2. Запустить `cargo run -- doctor`.
3. При необходимости временно включить mock backend.

## LibreOffice unavailable

Симптомы: код LIBREOFFICE_NOT_AVAILABLE.

Действия:

1. Установить LibreOffice в runtime.
2. Проверить PATH.
3. Повторить обработку.

## Queue full

Симптомы: API возвращает QUEUE_FULL, readyz=degraded.

Действия:

1. Увеличить job_queue_capacity.
2. Увеличить max_concurrent_jobs.
3. Масштабировать сервис горизонтально.

## High latency

1. Проверить benchmark и latency p95/p99.
2. Проверить batch settings и provider профиль.
3. Уменьшить размер входных документов по лимитам.

## Too many partial documents

1. Проверить warnings/errors в model.json.
2. Проверить доступность OCR/converters.
3. Сравнить с regression snapshots.

## PDF render failures

1. Проверить входной PDF на целостность.
2. Убедиться в доступности renderer/tooling.
3. Применить fallback маршрут.

## OOM prevention

1. Снизить max_file_size_mb и max_pages_per_document.
2. Ограничить concurrency.
3. Установить контейнерные memory limits.

## Disk full

1. Очистить output/data директории.
2. Включить ротацию и TTL для старых результатов.
3. Настроить мониторинг свободного места.
