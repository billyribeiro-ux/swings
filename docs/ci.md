# CI Reference

This document describes every GitHub Actions workflow in
`.github/workflows/` — what it runs, what failures look like, and how to
reproduce the checks on a local machine before pushing.

All workflows pin third-party actions to a version tag or commit SHA and
set `concurrency` so stale PR pushes cancel in-flight runs. Permissions
are minimal: `contents: read` by default, `security-events: write` only on
jobs that upload SARIF.

## `ci.yml` — baseline quality gates

### Jobs

| Job                | What it does                                                                                                                        |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| `frontend`         | `pnpm install --frozen-lockfile` → `pnpm ci:frontend` (`gen:types` + `check` + `lint` + `ci:seo` + `test:unit` + `build`).          |
| `backend`          | `cargo fmt --check` → `cargo clippy -D warnings` → `cargo test` → `cargo build`. Rust pinned to **1.93** via `rust-toolchain.toml`. |
| `backend-coverage` | Informational. Runs `cargo llvm-cov --workspace --lcov` and uploads `lcov.info` as a build artefact. No gate.                       |
| `e2e-smoke`        | Runs the `smoke-chromium` Playwright project against `pnpm build && pnpm preview` (frontend only, no backend required).             |

### Reproduce locally

```bash
pnpm ci:frontend
pnpm ci:backend
cargo llvm-cov --manifest-path backend/Cargo.toml --workspace --lcov --output-path backend/lcov.info
```

If `cargo-llvm-cov` is not installed:

```bash
cargo install cargo-llvm-cov --locked
# or, as a fallback: `cargo install cargo-tarpaulin --locked`
```

## `security.yml` — dependency & filesystem security

Runs on every PR, every push to `main`/`master`, and nightly at 03:00 UTC.

### Jobs

- **`cargo-audit`** — installs `cargo-audit` via `cargo-binstall` and runs
  `cargo audit --deny warnings` against `backend/Cargo.lock`. Advisory DB
  pinned to `https://github.com/rustsec/advisory-db`. Fails on any
  advisory, including unmaintained crates.
- **`pnpm-audit`** — runs `pnpm audit --audit-level=high` (blocking) and
  `pnpm audit --audit-level=moderate` (informational, `continue-on-error`).
- **`trivy-fs`** — `aquasecurity/trivy-action@0.35.0` in fs-scan mode,
  severity `HIGH,CRITICAL`, scanners limited to `secret,misconfig` (cargo
  / pnpm ecosystems are covered by the dedicated audit jobs above),
  respects `.trivyignore` at repo root, uploads SARIF to GitHub Code
  Scanning when available. Bump in lockstep with the workflow file —
  if you change the action version, update this paragraph too.

### Reproduce locally

```bash
cargo install cargo-audit --locked
cargo audit --manifest-path backend/Cargo.toml --deny warnings

pnpm audit --audit-level=high
pnpm audit --audit-level=moderate

# https://aquasecurity.github.io/trivy/latest/getting-started/installation/
trivy fs --severity HIGH,CRITICAL --ignorefile .trivyignore .
```

### Failure modes

- `cargo audit` fails → a new RustSec advisory dropped; either upgrade the
  offending crate or add a vetted `ignore` entry in
  `backend/audit.toml` (follow-up work — do not silence in the workflow).
- `pnpm audit` high/critical fails → `pnpm up <pkg>` or patch via
  `pnpm.overrides`.
- `trivy-fs` fails → inspect the SARIF artefact (or the job log) for the
  offending file path and rule. Misconfigurations in Dockerfiles and
  config-file secrets are the most common hits.

## `sql-lint.yml` — Postgres migration hygiene

Runs only on changes under `backend/migrations/**` (and on edits to the
`.sqlfluff` config itself).

- Installs `sqlfluff==3.2.5` via `pip install --user`.
- `sqlfluff lint backend/migrations --dialect postgres`.

The rule set in `.sqlfluff` deliberately excludes capitalisation and
layout rules so existing DDL does not trip on whitespace; the active
rules cover genuinely broken SQL and semantic anti-patterns.

### Reproduce locally

```bash
pipx install sqlfluff==3.2.5
# or: pip install --user sqlfluff==3.2.5
sqlfluff lint backend/migrations --dialect postgres
```

## `openapi-drift.yml` — OpenAPI contract

Runs on every PR and every push to `main`/`master`.

Two assertions:

1. `cargo test --test openapi_snapshot` — fails if the Rust source's
   `utoipa::OpenApi` derived spec diverges from the committed
   `backend/tests/snapshots/openapi.json`.
2. `pnpm gen:types` + `git diff --exit-code src/lib/api/schema.d.ts` —
   fails if the generated TypeScript types are stale relative to the
   snapshot.

### Failure modes

- Snapshot drift → run
  `UPDATE_OPENAPI_SNAPSHOT=1 cargo test --manifest-path backend/Cargo.toml --test openapi_snapshot`
  locally and commit the resulting JSON.
- Schema drift → run `pnpm gen:types` locally and commit
  `src/lib/api/schema.d.ts`.

### Reproduce locally

```bash
cargo test --manifest-path backend/Cargo.toml --test openapi_snapshot
pnpm gen:types
git diff --exit-code src/lib/api/schema.d.ts
```

## Configuration files

- `.trivyignore` — directories the filesystem scan skips (`node_modules`,
  `backend/target`, `.svelte-kit`, build outputs).
- `.sqlfluff` — sqlfluff configuration scoped to Postgres DDL.

## Toolchain pinning

- **Rust**: `rust-toolchain.toml` at repo root pins the compiler channel.
  CI workflows (`ci.yml`, `security.yml`, `openapi-drift.yml`) call
  `dtolnay/rust-toolchain@1.93` to match. The `Dockerfile` uses
  `rust:1.93-slim-bookworm`. Bump all three in a single PR.
- **Rust edition**: `2021` (declared in `backend/Cargo.toml`). Do not
  claim "2024" in docs — that edition is not stabilised for this
  project.
- **Node**: `.nvmrc` at repo root is the source of truth; all CI
  `actions/setup-node` steps read it via `node-version-file: .nvmrc`.

## Conventions for new workflows

- Pin actions to a tag (`@vN`) or SHA. Avoid `@master` / `@main`.
- Set `concurrency.group = ${{ github.workflow }}-${{ github.ref }}` and
  `cancel-in-progress: true` at the workflow scope.
- Start with `permissions: contents: read`; escalate per-job only if a
  step needs it (`security-events: write` for SARIF, `pull-requests: write`
  for comment bots).
- Cache Rust with `Swatinem/rust-cache@v2` (distinct `shared-key` per
  workflow to avoid cache thrash), Node via `actions/setup-node@v4`'s
  built-in `cache: pnpm`.
