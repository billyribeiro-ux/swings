# syntax=docker/dockerfile:1.7
#
# Production image for the swings-api Rust service.
#
# This is the *single* Dockerfile for the backend. It is consumed by:
#   * Railway       — build context = repo root (see `backend/railway.toml`)
#   * docker compose — build context = repo root (see `docker-compose.yml`)
#
# Build context is always the repo root so that the `COPY backend/...`
# instructions resolve consistently across PaaS targets. There is no
# secondary `backend/Dockerfile`; keep this file as the single source of
# truth and harden here.
#
# Security posture (aligned with Trivy DS-* rules):
#   * Pinned base images (no `:latest`) for reproducible builds.
#   * `--no-install-recommends` on every apt install to minimise attack
#     surface and image size.
#   * Multi-stage build — the runtime image never sees the Rust toolchain
#     or /var/cache/apt leftovers.
#   * Runs as a dedicated non-root `app` user (UID 10001); /app and
#     /app/uploads are owned by that user so writes succeed without root.
#   * No shell in the final command — the binary is the entrypoint.

# ─── Stage 1: builder ────────────────────────────────────────────────────
FROM rust:1.93-slim-bookworm AS builder

# `curl` + `ca-certificates` are required by `utoipa-swagger-ui`'s build
# script, which downloads the Swagger UI assets from GitHub at compile
# time. Without `curl` the build script panics with
# `failed to download Swagger UI: ... \`curl\` command not found`.
# The crate's alternative path (the `reqwest` feature) drags in a TLS
# stack we don't otherwise need at build time, so installing `curl` is
# the smaller delta.
#
# `pkg-config` + `libssl-dev` were intentionally removed when Stripe
# moved to the in-tree `reqwest`-rustls wrapper (see
# `backend/src/stripe_api.rs`). No direct or transitive crate now pulls
# `openssl-sys`; verify with `cargo tree --target all --invert openssl-sys`
# before re-introducing OpenSSL.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        curl \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Dependency-only layer — `cargo build` is cached until Cargo.{toml,lock}
# change, which is the expensive step in a clean CI runner.
COPY backend/Cargo.toml backend/Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release 2>/dev/null || true \
    && rm -rf src

# Real sources. The root `.dockerignore` keeps the context small even
# though the build context is the entire repo.
COPY backend/ .
RUN touch src/main.rs && cargo build --release

# ─── Stage 2: runtime ────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 app \
    && useradd  --system --uid 10001 --gid app --home-dir /app --shell /usr/sbin/nologin app

WORKDIR /app

COPY --from=builder --chown=app:app /app/target/release/swings-api ./swings-api
COPY --from=builder --chown=app:app /app/migrations ./migrations

RUN mkdir -p uploads && chown -R app:app /app

USER app

EXPOSE 3001

# Liveness probe — `handlers::health::router()` mounts `/health` (not under
# `/api`, see `backend/src/main.rs`). The endpoint never touches Postgres,
# so a failing healthcheck means the Axum process itself is wedged.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -fsS http://127.0.0.1:3001/health || exit 1

CMD ["./swings-api"]
