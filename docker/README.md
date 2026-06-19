# Docker запуск

## Варианты образов

- minimal: только бинарь document_parser и базовые runtime зависимости.
- full-converters: minimal + LibreOffice/Pandoc/Poppler (опционально, добавляется в кастомном Dockerfile).

## Локальный запуск

```bash
docker compose -f docker/docker-compose.local.yml up --build
```

## Примечания

- Порт API: 8080
- Рабочие каталоги внутри контейнера:
  - /app/configs
  - /app/data
- Для production рекомендуется изолировать контейнер и ограничивать права доступа.
