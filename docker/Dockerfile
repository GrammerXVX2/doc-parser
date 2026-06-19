FROM rust:1.90-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY configs ./configs
COPY docs ./docs
COPY schemas ./schemas
COPY examples ./examples
COPY testdata ./testdata
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
RUN useradd -m -u 10001 appuser
WORKDIR /app

COPY --from=builder /build/target/release/document_parser /usr/local/bin/document_parser
COPY --from=builder /build/configs /app/configs

RUN mkdir -p /app/data /app/configs && chown -R appuser:appuser /app
USER appuser

EXPOSE 8080
CMD ["document_parser", "serve", "--config", "/app/configs/profiles/api.jsonc"]
