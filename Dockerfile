# Dockerfile for miniflux-reader-rs v0.2.0
#
# Multi-stage build:
#   1. builder: rust:1.92-slim + cargo-leptos → produces single static-ish binary
#   2. runtime: debian:bookworm-slim + ca-certificates + binary → ~50MB image
#
# Build:  docker build -t miniflux-reader-rs:v0.2.0 .
# Run:    docker run -p 8083:8083 -v $(pwd)/rust-config.json:/app/rust-config.json \
#                 -v $(pwd)/rust-data:/app/rust-data miniflux-reader-rs:v0.2.0

# ---------- Stage 1: builder ----------
FROM rust:1.92-slim-bookworm AS builder

# System deps: ssl (rustls needs certs), sqlite3 (sqlx compile-time),
# pkg-config, make. Leptos SSR doesn't need node.js.
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    ca-certificates \
    make \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-leptos (the Leptos 0.7 build tool) at a pinned version.
# cargo-leptos handles SSR + WASM hydration + asset bundling in one command.
RUN cargo install cargo-leptos --version 0.2.24 --locked

WORKDIR /build

# Copy manifest + lock first for layer caching (deps rarely change vs source).
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ ./crates/
COPY migrations/ ./migrations/

# Build release binary. cargo-leptos produces target/release/http-server
# (SSR binary) + target/site/pkg/*.wasm (hydration client, embedded).
ENV RUSTFLAGS="--cfg getrandom_backend=\"wasm_js\""
RUN cargo leptos build --release

# ---------- Stage 2: runtime ----------
FROM debian:bookworm-slim AS runtime

# Runtime deps: ca-certificates (HTTPS to RSS feeds / translate / tts),
# libsqlite3-0 (sqlx runtime), tini (PID 1 signal handling).
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libsqlite3-0 \
    tini \
    && rm -rf /var/lib/apt/lists/*

# Non-root user for security (avoid running as root in container).
RUN useradd --create-home --uid 1000 --shell /bin/false appuser

WORKDIR /app

# Copy the release binary + Leptos site assets (embedded into binary via
# include_dir, but we copy assets/ for runtime inject script overrides).
COPY --from=builder /build/target/release/http-server /app/http-server
COPY --from=builder /build/crates/http-server/assets /app/assets
COPY --from=builder /build/rust-config.example.json /app/rust-config.example.json

# Create data directories with correct ownership.
RUN mkdir -p /app/rust-data /app/rust-epub-books && \
    chown -R appuser:appuser /app

USER appuser

EXPOSE 8083

# tini handles SIGTERM properly so cargo leptos server shuts down cleanly.
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/app/http-server", "/app/rust-config.json"]
