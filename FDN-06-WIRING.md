# FDN-06 — `common::*` utility modules — integrator wiring

This subsystem adds `backend/src/common/{money,geo,ua,html}.rs` + a `mod.rs`
root. It is **dependency-adding** and must not be compiled until the integrator
applies the Cargo.toml changes below and registers the module in `main.rs`.

The modules themselves are self-contained — no other file in `backend/src/*`
is modified by this patch.

---

## 1. `backend/Cargo.toml` additions

Add the following to the `[dependencies]` table. These are the pinned versions
used during FDN-06's verification gate; newer patch-compatible releases are
fine.

```toml
# Geo (GeoLite2-Country reader)
maxminddb = "0.25"

# User-agent parsing + cache
woothee = "0.13"
moka    = { version = "0.12", features = ["sync"] }

# HTML sanitization
ammonia = "4"
```

Nothing needs to be removed. The additions are additive and do not touch the
existing feature flags on `sqlx`, `lettre`, `aws-sdk-s3`, etc.

### Transitive-conflict note

During verification, `cargo build` was run with the full dependency graph
(including `aws-sdk-s3 = "1"` and `sqlx = "0.8"`). No version conflicts were
observed. If a future dependency bump produces a conflict with `html5ever`
(ammonia's transitive), pin `ammonia` to the last 4.x and file a follow-up.

---

## 2. `backend/src/main.rs` — module registration

Insert the new module declaration alongside the other `mod` statements at the
top of `main.rs`. Alphabetical placement:

```rust
mod common;
mod config;
mod db;
// ... existing declarations unchanged
```

No other change to `main.rs` is required. The `common::*` functions are
standalone utilities and do not need to be wired into the `Router` or
`AppState` at integration time; handlers that want them will import directly,
e.g. `use crate::common::money::Money;`.

---

## 3. Environment variables

| Name               | Required? | Purpose                                                                                             |
| ------------------ | --------- | --------------------------------------------------------------------------------------------------- |
| `MAXMIND_DB_PATH`  | optional  | Filesystem path to a GeoLite2-Country `.mmdb` file. If unset or unreadable, `geo::country_from_ip` logs one warning and subsequently returns `None`. |

No other env vars are introduced. `common::html` and `common::ua` are entirely
in-memory; `common::money` has no external config.

---

## 4. Integrator verification gate

After applying the Cargo.toml and `main.rs` edits, run from `backend/`:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

All three must pass with zero warnings. FDN-01's 20 existing `error::tests::*`
must continue to pass; the new modules add:

- `common::money::tests` — 22 tests
- `common::geo::tests`   — 11 tests
- `common::ua::tests`    — 6 tests
- `common::html::tests`  — 15 tests

Total new tests: 54.

---

## 5. Rollback

Reverting `backend/Cargo.toml` (to drop the four new crates) and removing
`mod common;` from `main.rs` is sufficient. Leaving the `backend/src/common/`
files in the tree is harmless because nothing references them, but they will
be dead code, so a clean revert should also `rm -rf backend/src/common/`.
