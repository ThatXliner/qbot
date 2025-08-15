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
ENV RUST_LOG=info
# IMPORTANT: service discovery by container name inside the task
ENV OLLAMA_URL=http://ollama:11434

ENTRYPOINT ["/app/qbot"]
