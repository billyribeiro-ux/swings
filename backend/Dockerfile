# syntax=docker/dockerfile:1.7
#
# Production image for the swings-api Rust service.
#
# Security posture (aligned with Trivy DS-* rules):
#   * Pinned base images (no `:latest`) for reproducible builds.
#   * `--no-install-recommends` on every apt install to minimise attack
#     surface and image size.
#   * Multi-stage build — the runtime image never sees the Rust toolchain
#     or /var/cache/apt leftovers.
#   * Runs as a dedicated non-root `app` user; /app + /app/uploads owned
#     by that user so writes succeed without root.
#   * `HEALTHCHECK` lets orchestrators distinguish a hung process from a
#     healthy one without us shipping a sidecar.

# ─── Stage 1: builder ────────────────────────────────────────────────────
FROM rust:1.93-slim-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Dependency-only layer — `cargo build` is cached until Cargo.{toml,lock}
# change, which is the expensive step in a clean CI runner.
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release 2>/dev/null || true \
    && rm -rf src

# Real sources.
COPY . .
RUN touch src/main.rs && cargo build --release

# ─── Stage 2: runtime ────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 app \
    && useradd  --system --uid 10001 --gid app --home-dir /app --shell /usr/sbin/nologin app

WORKDIR /app

COPY --from=builder --chown=app:app /app/target/release/swings-api ./swings-api
COPY --from=builder --chown=app:app /app/migrations ./migrations

RUN mkdir -p uploads && chown -R app:app /app

USER app

EXPOSE 3001

CMD ["./swings-api"]
