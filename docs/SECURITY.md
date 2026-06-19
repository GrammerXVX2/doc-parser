# Security

## Threat model

Сервис принимает недоверенные документы разных форматов, включая архивные контейнеры и бинарные офисные форматы.

## Риски форматов

- PDF: поврежденные структуры, oversized pages
- OOXML: zip slip, archive bomb, слишком много entries
- Images: экстремальные размеры
- Legacy DOC/RTF: риски внешних конвертеров

## Риски внешних конвертеров

LibreOffice/Pandoc запускаются как внешние процессы с timeout и лимитами stdout/stderr.

## Sandbox policy

- timeout для каждого процесса
- ограничение stdout/stderr
- отдельный temp workspace
- без shell-интерполяции аргументов

## Текущие лимиты

См. SecurityConfig и SecurityLimits:

- max_file_size_mb
- max_pages_per_document
- max_extracted_assets_mb
- max_image_width_px/max_image_height_px
- max_archive_entries
- max_archive_total_uncompressed_mb

## Известные ограничения

- Нет изоляции на уровне VM/namespace по умолчанию
- Нет встроенного malware scanning

## Рекомендации для production

- запускать сервис в изолированном контейнере
- ограничивать CPU/RAM/disk quotas
- ограничивать сетевой доступ
- использовать read-only rootfs и non-root пользователя
