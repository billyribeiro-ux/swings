# syntax=docker/dockerfile:1.7
#
# Root-level Dockerfile used by PaaS deployments (e.g. Railway, Fly, Render)
# whose build context is the repo root. Functionally identical to
# `backend/Dockerfile`; the only delta is the `COPY` paths since this
# variant reads from the `backend/` subdirectory.
#
# Keep the two files in sync when hardening one of them. See
# `backend/Dockerfile` for the security-posture rationale.

# ─── Stage 1: builder ────────────────────────────────────────────────────
FROM rust:1.93-slim-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY backend/Cargo.toml backend/Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release 2>/dev/null || true \
    && rm -rf src

COPY backend/ .
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
