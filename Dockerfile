FROM rust:1.89 as builder

LABEL org.opencontainers.image.source=https://github.com/ThatXliner/qbot
# Set the working directory
WORKDIR /app

# Copy the Cargo files first for better Docker layer caching
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
# Build dependencies (this layer will be cached)
RUN cargo build --release --jobs 1

RUN apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Runtime
FROM debian:trixie-slim
WORKDIR /app
COPY --from=builder /app/target/release/qbot /app/qbot
COPY templates/ ./templates/

# Slim doesn't contain trusted certificates

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

ENV RUST_LOG=info
# IMPORTANT: service discovery by container name inside the task
ENV OLLAMA_URL=http://0.0.0.0:11434

ENTRYPOINT ["/app/qbot"]
