//! Build-time metadata capture for the `/version` endpoint.
//!
//! We embed three facts about the build into the binary via `rustc-env`:
//!
//! * `GIT_SHA` — short commit SHA, or `unknown` when the build happens outside
//!   a git checkout (Docker image from a tarball, `cargo publish` extraction,
//!   etc.).
//! * `GIT_SHA_LONG` — full commit SHA or `unknown`, useful for issue logs.
//! * `BUILD_TIME` — ISO-8601 UTC timestamp of the build. Operators use this
//!   to disambiguate two builds from the same SHA.
//!
//! This script is deliberately tolerant — if `git` is unavailable (vendored
//! build, missing `.git` directory inside a container), we fall through to
//! `unknown` rather than failing the build. Production builds should also
//! set `GIT_SHA` / `GIT_SHA_LONG` via the CI pipeline as a belt-and-braces
//! fallback; whatever the CI sets wins.
//!
//! This file is consumed at compile time by `backend/src/handlers/version.rs`
//! through `env!("GIT_SHA")` / `env!("GIT_SHA_LONG")` / `env!("BUILD_TIME")`.

use std::process::Command;

fn main() {
    // If CI already injected these, respect them verbatim — CI has stronger
    // guarantees about what commit is actually being shipped than a local
    // git invocation does (it sees PR-merge commits, for example).
    let git_sha = std::env::var("GIT_SHA")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| run_git(&["rev-parse", "--short=12", "HEAD"]))
        .unwrap_or_else(|| "unknown".to_string());

    let git_sha_long = std::env::var("GIT_SHA_LONG")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| run_git(&["rev-parse", "HEAD"]))
        .unwrap_or_else(|| "unknown".to_string());

    let build_time = std::env::var("BUILD_TIME")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            // ISO-8601 UTC: "2024-06-17T15:04:05Z". No `chrono` in `build.rs`
            // to keep the build-script dependency graph empty; we use the
            // `SOURCE_DATE_EPOCH` env contract when available (reproducible
            // builds) and fall back to the current system clock otherwise.
            let secs = std::env::var("SOURCE_DATE_EPOCH")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or_else(|| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                });
            format_epoch(secs)
        });

    println!("cargo:rustc-env=GIT_SHA={}", git_sha);
    println!("cargo:rustc-env=GIT_SHA_LONG={}", git_sha_long);
    println!("cargo:rustc-env=BUILD_TIME={}", build_time);

    // Don't re-run on arbitrary source changes — tracking `.git/HEAD` and
    // the active ref is enough to capture "did the commit change?" without
    // invalidating the cache on every file edit.
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-env-changed=GIT_SHA");
    println!("cargo:rerun-if-env-changed=GIT_SHA_LONG");
    println!("cargo:rerun-if-env-changed=BUILD_TIME");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
}

fn run_git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Format a UNIX epoch as `YYYY-MM-DDTHH:MM:SSZ` using Howard Hinnant's
/// civil_from_days algorithm. We do this by hand so `build.rs` stays
/// dependency-free; the math is well-understood and tested against the
/// reference implementation at https://howardhinnant.github.io/date_algorithms.html.
fn format_epoch(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let h = rem / 3_600;
    let m = (rem / 60) % 60;
    let s = rem % 60;

    let (y, mo, d) = civil_from_days(days);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, d, h, m, s)
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
