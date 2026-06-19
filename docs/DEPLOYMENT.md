# Deployment

## Локальный бинарь

```bash
cargo run -- serve --config configs/profiles/api.jsonc
```

## Docker

```bash
docker build -f docker/Dockerfile -t document-parser .
docker run --rm -p 8080:8080 -v $(pwd)/configs:/app/configs -v $(pwd)/data:/app/data document-parser
```

## systemd пример

```ini
[Unit]
Description=document-parser service
After=network.target

[Service]
WorkingDirectory=/opt/document-parser
ExecStart=/opt/document-parser/document_parser serve --config /opt/document-parser/configs/profiles/api.jsonc
Restart=always
User=docparser
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
```

## Environment variables

- RUST_LOG
- PATH (для внешних конвертеров)

## Config files

- configs/pipeline.config.jsonc
- configs/format_routing.config.jsonc
- configs/profiles/*.jsonc
