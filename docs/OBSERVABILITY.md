# Observability

## Логи

Логи пишутся через tracing с request_id/method/route/status/duration.

## Tracing fields

- request_id
- route
- method
- status
- duration_ms
- job_id/document_id (в worker span)

## Метрики

Prometheus text endpoint `/metrics`:

- documents_submitted_total
- jobs_queued
- documents_completed_total
- documents_partial_total
- documents_failed_total
- document_processing_duration_ms
- job_queue_wait_ms
- job_processing_ms

## Prometheus endpoint

- путь: `/metrics`
- формат: text/plain; version=0.0.4

## Health/readiness

- `/healthz`: жив ли процесс
- `/readyz`: учитывает queue/storage/config/model readiness
