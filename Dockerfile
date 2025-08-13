# Use the official Rust image as the base
FROM rust:1.89 as builder

# Set the working directory
WORKDIR /app

# Copy the Cargo files first for better Docker layer caching
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
COPY templates/ ./templates/
# Build dependencies (this layer will be cached)
RUN cargo build --release --jobs 1

RUN apt-get clean && \
  rm -rf /var/lib/apt/lists/*

# Runtime
FROM alpine:3.22.1
WORKDIR /app
COPY --from=builder /app/target/release/qbot /app/qbot
ENV RUST_LOG=info
# IMPORTANT: service discovery by container name inside the task
ENV OLLAMA_URL=http://ollama:11434
CMD ["./qbot"]
