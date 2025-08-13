# Use the official Rust image as the base
FROM rust:1.87-slim as builder

# Install system dependencies needed for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy the Cargo files first for better Docker layer caching
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
# Build dependencies (this layer will be cached)
RUN cargo build --release

# Build stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/qbot /app/qbot
ENV RUST_LOG=info
# IMPORTANT: service discovery by container name inside the task
ENV OLLAMA_URL=http://ollama:11434
CMD ["./qbot"]
