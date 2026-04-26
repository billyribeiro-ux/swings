# FDN-TESTHARNESS — Backend integration-test harness

This document describes the in-process integration-test harness under
`backend/tests/support/`, what the crate needs on top of the current
`Cargo.toml` to consume it, and how to run it.

The harness is shipped as new files only — it does **not** modify any
existing code, `Cargo.toml`, or migrations. The integrator wiring in §2
should be applied by the agent that owns `backend/Cargo.toml` in the same
PR that lands integration tests written against the harness.

---

## 1. What the harness provides

```
backend/tests/
├── support/
│   ├── mod.rs       Public re-exports: TestApp, TestUser, TestResponse, …
│   ├── app.rs       TestApp + router mirror + rate-limit IP allocator
│   ├── db.rs        TestDb — schema-per-test Postgres isolation
│   ├── error.rs     TestAppError — typed harness errors via thiserror
│   ├── response.rs  TestResponse + AssertProblem — RFC 7807 assertions
│   └── user.rs      seed_user / seed_admin with JWT minting
└── example_auth_flow.rs  Exemplar integration test
```

### Schema-level isolation (rationale)

Because the crate's `Cargo.toml` is off-limits to this harness, we cannot
take on `sqlx-testcontainers`, `#[sqlx::test]`, or any CREATE DATABASE
tooling. Instead, each `TestDb::new`:

1. Connects to `DATABASE_URL_TEST` (falls back to `DATABASE_URL`).
2. Creates a fresh `test_<hex32>` schema on a one-shot admin pool.
3. Opens a scoped pool whose `after_connect` hook `SET search_path` pins
   every pooled connection to that schema.
4. Runs the committed `sqlx::migrate!("./migrations")` set against the
   pool — every migration lands under the sandbox schema.
5. On `Drop`, best-effort `DROP SCHEMA … CASCADE`.

Application code in `handlers::*` / `db.rs` uses unqualified relation
names (`FROM users`, `FROM refresh_tokens`), so with `search_path` pinned
the identical SQL the production binary runs is what the tests run —
every subsystem landing in Phase 4 benefits for free.

Alternatives considered and rejected:

| Option                             | Why not                                                                                                                                                      |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| DB-per-test (`CREATE DATABASE`)    | Requires elevated privileges many shared Postgres hosts won't grant; slower to spin up.                                                                      |
| `#[sqlx::test]` + `macros` feature | Requires a `Cargo.toml` edit; our scope forbids that.                                                                                                        |
| `testcontainers` + Docker-in-CI    | Docker not guaranteed in every dev environment; our scope explicitly forbids it.                                                                             |
| Transaction-per-test               | Handlers commit via nested transactions, so rollback-based isolation loses any cross-request state — breaks multi-request flows like refresh-token rotation. |

### Router mirroring

`TestApp::new` constructs the same `Router` tree `main.rs` does at
startup:

```
/api/auth/*            auth::router
/api/analytics/*       analytics::router
/api/admin/*           admin::router
/api/admin/blog/*      blog::admin_router
/api/admin/courses/*   courses::admin_router
…
/api/member/*          member::router + courses::member_router
/api/webhooks/*        webhooks::router
```

**Maintenance:** whenever a new `.nest(…)` / `.merge(…)` is added in
`backend/src/main.rs`, mirror it in `backend/tests/support/app.rs::build_router`.
A future cleanup (follow-up subsystem, not this scope) should extract
`pub fn build_router(state: AppState) -> Router` from `main.rs` so the
harness calls the single source of truth.

### Rate-limit handling

`tower_governor::SmartIpKeyExtractor` reads `X-Forwarded-For` first. Each
`TestApp` allocates a UUID-seeded `10.x.y.z` and stamps it on every
request, so IP-keyed governor buckets do not overlap between concurrent
tests.

### Auth

`support::user::seed` hashes passwords with Argon2 defaults (matching
`handlers::auth::register`) and mints JWTs with the same secret + claims
layout the production `AuthUser` extractor accepts. `TestUser.password`
is retained so tests can drive real `POST /api/auth/login` calls.

### RFC 7807 ergonomics

`TestResponse::assert_problem(AssertProblem { status, type_suffix, title })`
asserts the shape produced by `backend/src/error.rs` (including the
`application/problem+json` content type). Schema extras like `instance`
and `correlation_id` are ignored so tests stay stable across
FDN-01/FDN-04 changes to the Problem body.

---

## 2. Integrator TODOs (Cargo.toml additions)

None of the following are made by this harness. They must be applied by
the agent that owns `backend/Cargo.toml` before any additional
integration tests land. **The exemplar `example_auth_flow.rs` already
compiles and runs (skipping) under the current, unchanged `Cargo.toml`.**

### Required for new tests that also want `#[sqlx::test]`

The sqlx `macros` feature is already enabled transitively (it is in
sqlx's default feature set, and this crate does not set
`default-features = false`). No Cargo.toml changes are required unless
a downstream test opts into compile-time query macros — at which point
add `"macros"` explicitly for documentation clarity:

```toml
sqlx = { version = "0.8", features = [
    "runtime-tokio", "tls-rustls", "postgres", "uuid", "chrono",
    "migrate", "json", "rust_decimal",
    "macros", # used by #[sqlx::test] and query!/query_as!; harness itself does not use it.
] }
```

### Already present (confirmed)

- `tempfile = "3"` — used for `MediaBackend::Local` uploads tempdir.
- `tower::util::ServiceExt::oneshot` — enabled transitively via
  `tower_governor → tonic → tower/util` feature. If that transitive path
  is ever removed, add `tower = { version = "0.5", features = ["util"] }`
  explicitly.

### Not required (explicitly out of scope)

- No `testcontainers`, no `sqlx-testcontainers`, no Docker spawn logic.
- No dev-dependency migration — the harness compiles inside the existing
  `[dependencies]` block because integration tests are compiled as
  separate crates inside the same package.

---

## 3. Required environment variables

| Var                 | Required? | Description                                                                                                         |
| ------------------- | --------- | ------------------------------------------------------------------------------------------------------------------- |
| `DATABASE_URL_TEST` | preferred | Postgres URL where the harness has `CREATE SCHEMA` privileges.                                                      |
| `DATABASE_URL`      | fallback  | Used if `DATABASE_URL_TEST` is absent.                                                                              |
| `JWT_SECRET`        | optional  | Overrides the per-process test secret (needed only when a test cross-verifies a token produced by another process). |
| `KEEP_TEST_SCHEMA`  | optional  | Set to `1` to skip `DROP SCHEMA` on test teardown — lets you inspect fixture state after a failure.                 |

If neither DB URL is set, `TestApp::try_new` returns `None` and the test
function should `return` early. The `example_auth_flow` exemplar shows
the idiom.

---

## 4. How to run

### One-liner against `docker compose`

```bash
docker compose -f backend/docker-compose.yml up -d db
DATABASE_URL_TEST=postgres://postgres:postgres@localhost:5433/swings \
  cargo test --manifest-path backend/Cargo.toml
```

### Running just the exemplar

```bash
DATABASE_URL_TEST=postgres://postgres:postgres@localhost:5433/swings \
  cargo test --manifest-path backend/Cargo.toml --test example_auth_flow
```

### Running without a DB (skip path)

```bash
cargo test --manifest-path backend/Cargo.toml
```

The 74 unit tests and the OpenAPI snapshot test run; any integration
test that calls `TestApp::try_new` detects the missing DB URL and
returns successfully with a skip notice on stderr.

---

## 5. Known limitations + cleanup

### Parallel execution

`cargo test` runs integration tests in parallel by default. Each
`TestApp` creates one schema, so a run with N parallel tests will see
N transient schemas. In the happy path each is dropped in `TestDb::drop`;
a panicking test may leak its schema.

### Orphan-schema cleanup

Run periodically on the test database:

```sql
DO $$
DECLARE s text;
BEGIN
    FOR s IN
        SELECT nspname FROM pg_namespace WHERE nspname LIKE 'test\_%' ESCAPE '\'
    LOOP
        EXECUTE 'DROP SCHEMA IF EXISTS ' || quote_ident(s) || ' CASCADE';
    END LOOP;
END
$$;
```

The `LIKE 'test\_%'` pattern matches the UUID-suffixed names the harness
generates without catching unrelated schemas that might happen to
contain an underscore.

### Router mirroring is duplicated

`build_router` in `support::app` mirrors `main.rs`. Any new subsystem
(FDN-04 outbox admin routes, forms, consent, popups extensions) MUST
update the harness in lockstep or integration tests for those routes
will 404.

**Long-term fix (deferred to a follow-up subsystem):** extract
`pub fn build_router(state: AppState) -> Router<()>` in
`backend/src/main.rs` (or `backend/src/lib.rs`) and call it from both
the binary and the harness.

### Rate limits still apply

Production rate limits (login 5/min, register 10/hour, forgot-password
3/hour, analytics 120/sec) remain active in tests. The harness
inoculates each `TestApp` with a unique IP so bursts do not bleed
between tests, but a single test that fires more than the burst quota
at the same route will hit 429 just like a real client.

### Email is stubbed

`AppState.email_service = None`. Handlers that fall back to logging the
reset URL (per `handlers::auth::forgot_password`) will log but not send.
Any Phase 4 subsystem that hard-requires a working `EmailService` must
inject a fake — see `email::EmailService::new`'s signature and the
`swings_api` re-exports.

### R2 is unused

`MediaBackend::Local` is always selected; `R2Storage::from_env()` is not
invoked in tests. Tests that need the R2 code path should spin up a
local MinIO or moto-S3 outside the harness.

---

## 6. Exemplar: `tests/example_auth_flow.rs`

```rust
mod support;

use axum::http::StatusCode;
use serde_json::json;
use support::{AssertProblem, TestApp};

#[tokio::test]
async fn auth_flow_happy_and_unhappy_paths() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed member");

    app.post_json(
        "/api/auth/login",
        &json!({ "email": user.email, "password": "wrong" }),
        None,
    )
    .await
    .assert_problem(AssertProblem {
        status: StatusCode::UNAUTHORIZED,
        type_suffix: "unauthorized",
        title: "Unauthorized",
    });

    let resp = app
        .post_json(
            "/api/auth/login",
            &json!({ "email": user.email, "password": user.password }),
            None,
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: serde_json::Value = resp.json().unwrap();
    let token = body["access_token"].as_str().unwrap();

    app.get("/api/auth/me", Some(token))
        .await
        .assert_status(StatusCode::OK);

    app.get("/api/auth/me", None).await.assert_problem(AssertProblem {
        status: StatusCode::UNAUTHORIZED,
        type_suffix: "unauthorized",
        title: "Unauthorized",
    });
}
```

---

## 7. Follow-ups / open items

1. **Extract `build_router` into the crate** so the harness imports the
   single source of truth instead of mirroring.
2. **Add a lightweight in-process email fake** (`EmailService::fake()` →
   captures sent bodies into a `Mutex<Vec<…>>`) for tests that assert on
   outbound email content.
3. **Migration advisory lock** inside `TestDb::new` so that parallel
   schema creates do not step on each other when the CI runner is
   heavily parallelized. Not needed at current parallelism levels.
4. **CI workflow**: add `backend-integration.yml` that spins up a
   Postgres service container and runs `cargo test` with
   `DATABASE_URL_TEST` wired up.
