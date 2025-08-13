# Use the official Rust image as the base
FROM rust:1.75-slim as builder

# Install system dependencies needed for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy the Cargo files first for better Docker layer caching
COPY Cargo.toml Cargo.lock ./
# Build dependencies (this layer will be cached)
RUN cargo build --release

# Copy the source code
COPY src/ ./src/
# Build the application
RUN cargo build --release

# Create the runtime image
FROM ollama:0.11.4

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1000 qbot

# Set the working directory
WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/qbot /app/qbot

# Change ownership to the qbot user
RUN chown qbot:qbot /app/qbot

# Switch to the non-root user
USER qbot

RUN ollama pull qwen3:1.7b

# Set environment variables with defaults
ENV RUST_LOG=info
ENV OLLAMA_URL=http://127.0.0.1:11434

# Run the application
CMD ["./qbot"]
