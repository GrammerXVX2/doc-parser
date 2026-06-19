FROM rust:1.89-bookworm

WORKDIR /workspace

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY configs ./configs
COPY docs ./docs
COPY schemas ./schemas
COPY examples ./examples
COPY tests ./tests
COPY testdata ./testdata

RUN cargo build --release

CMD ["./target/release/document_parser", "parse", "testdata/sample.html", "--output", "output"]
