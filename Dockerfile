FROM rust:1.89-trixie AS builder

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && dpkg --compare-versions "$(dpkg-query -W -f='${Version}' ca-certificates)" ge "20250419+deb13u1" \
    && rm -rf /var/lib/apt/lists/*

LABEL org.opencontainers.image.source=https://github.com/ThatXliner/qbot
# Set the working directory
WORKDIR /app

# Copy the Cargo files first for better Docker layer caching
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
# Build dependencies (this layer will be cached)
RUN cargo build --release --locked --jobs 1

RUN apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Runtime
FROM debian:trixie-slim
WORKDIR /app
COPY --from=builder /app/target/release/qbot /app/qbot
COPY templates/ ./templates/

# Slim doesn't contain trusted certificates

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3t64 \
    && dpkg --compare-versions "$(dpkg-query -W -f='${Version}' ca-certificates)" ge "20250419+deb13u1" \
    && rm -rf /var/lib/apt/lists/*

ENV RUST_LOG=info

ENTRYPOINT ["/app/qbot"]
