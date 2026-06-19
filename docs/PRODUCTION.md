# Production

## Обзор архитектуры

Сервис состоит из CLI/API слоя, очереди задач, пайплайна извлечения, подсистемы OCR и локального хранения результатов.

## Runtime profiles

- local: локальная разработка
- api: HTTP API профиль
- batch: пакетная обработка
- safe: консервативные лимиты
- gpu/triton/benchmark: профили производительности

## Рекомендуемые лимиты

- max_file_size_mb: ограничивать по SLA
- max_pages_per_document: ограничивать по типу документов
- max_processing_time_sec: контролировать деградации
- max_archive_entries/max_archive_total_uncompressed_mb: защита от archive-bomb

## Worker settings

Настраиваются через service profile: max_concurrent_jobs, job_queue_capacity.

## Storage layout

- input_dir: входящие документы
- output_dir: output/<document_id>/{model.json, markdown.md, plain_text.txt}

## Model loading

ONNX модели и charset проверяются в doctor-команде.

## OCR backend modes

- mock
- onnx
- triton (если сервер доступен)

При недоступности backend возможен fallback_to_mock.
