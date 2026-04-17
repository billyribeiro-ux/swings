//! JSON log formatter layer for `tracing_subscriber`.
//!
//! Production deployments (Railway, Vercel, Fly, etc.) ingest stdout line-by-
//! line into a structured log store. A JSON formatter keeps the ingest path
//! zero-copy — field names stay stable and operators can pivot on
//! `request_id` / `user_id` / `route` without regex-parsing a prose line.
//!
//! Format is selected at layer-construction time (startup) by
//! [`should_use_json`]; swapping formats in-flight is not supported because
//! the layer is installed once via `tracing_subscriber::registry().init()`.
//!
//! # Env vars
//!
//! * `LOG_FORMAT=json` forces JSON (overrides `APP_ENV`).
//! * `LOG_FORMAT=pretty` forces pretty (overrides `APP_ENV`).
//! * Otherwise JSON when `APP_ENV=production`, pretty otherwise.
//!
//! # Why a boxed layer?
//!
//! `fmt::Layer<_>.json()` and `fmt::Layer<_>.pretty()` have different static
//! types; we pick one at runtime and unify by boxing via
//! [`tracing_subscriber::Layer::boxed`]. The cost is a vtable dispatch per
//! event, which is negligible next to stdout I/O.
//!
//! # Wiring
//!
//! ```rust,ignore
//! use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
//! use swings_api::observability::tracing_json;
//!
//! tracing_subscriber::registry()
//!     .with(tracing_subscriber::EnvFilter::try_from_default_env()
//!         .unwrap_or_else(|_| "swings_api=debug,tower_http=debug".into()))
//!     .with(tracing_json::layer())
//!     .init();
//! ```

use std::env;

use tracing::Subscriber;
use tracing_subscriber::{fmt, registry::LookupSpan, Layer};

/// Build the `tracing` formatting layer. Caller composes with an
/// [`tracing_subscriber::EnvFilter`] + registry via
/// `registry().with(filter).with(layer()).init()`.
///
/// The returned layer is boxed via [`Layer::boxed`] so the JSON and pretty
/// paths can share a single return type while still being composable with
/// any downstream `Layered<…>` stack.
pub fn layer<S>() -> Box<dyn Layer<S> + Send + Sync + 'static>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    if should_use_json() {
        fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(false)
            .with_target(true)
            .with_file(false)
            .with_line_number(false)
            .with_thread_ids(false)
            .with_thread_names(false)
            .boxed()
    } else {
        fmt::layer()
            .pretty()
            .with_target(true)
            .with_file(false)
            .boxed()
    }
}

/// Decision logic for which formatter to install.
///
/// Priority (first-match-wins):
///
/// 1. `LOG_FORMAT=json` / `LOG_FORMAT=pretty` — explicit override.
/// 2. `APP_ENV=production` → JSON (structured log ingest).
/// 3. Otherwise → pretty (developer console).
fn should_use_json() -> bool {
    if let Ok(raw) = env::var("LOG_FORMAT") {
        match raw.trim().to_ascii_lowercase().as_str() {
            "json" => return true,
            "pretty" => return false,
            // Unknown value — fall through to env-based decision rather than
            // panic; an operator typo shouldn't brick startup.
            _ => {}
        }
    }
    env::var("APP_ENV")
        .map(|v| v.eq_ignore_ascii_case("production"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use tracing_subscriber::Registry;

    /// Env-var tests in the same process share a single mutable map; run
    /// them serially via this mutex so two tests can't race on `set_var`
    /// and see each other's intermediate state. The `OnceLock` avoids the
    /// `lazy_static` dep.
    fn env_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poison| {
                // A previous test panicked while holding the lock. The
                // subsequent test can still read env vars safely; take the
                // poisoned guard so we don't double-panic.
                poison.into_inner()
            })
    }

    /// Helper to run a closure with a scoped env override. Restores the
    /// original values (or removes them if absent) on drop so tests don't
    /// leak state to each other.
    struct EnvGuard {
        keys: Vec<(&'static str, Option<String>)>,
        _lock: MutexGuard<'static, ()>,
    }

    impl EnvGuard {
        fn set(keys: &[(&'static str, Option<&str>)]) -> Self {
            // Acquire the env-lock BEFORE reading/writing any env var so
            // concurrent tests see a consistent view.
            let lock = env_lock();
            let mut prev = Vec::with_capacity(keys.len());
            for (k, v) in keys {
                prev.push((*k, env::var(k).ok()));
                match v {
                    Some(val) => env::set_var(k, val),
                    None => env::remove_var(k),
                }
            }
            Self {
                keys: prev,
                _lock: lock,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (k, v) in &self.keys {
                match v {
                    Some(val) => env::set_var(k, val),
                    None => env::remove_var(k),
                }
            }
        }
    }

    #[test]
    fn log_format_json_forces_json() {
        let _g = EnvGuard::set(&[("LOG_FORMAT", Some("json")), ("APP_ENV", Some("dev"))]);
        assert!(should_use_json());
    }

    #[test]
    fn log_format_pretty_forces_pretty_even_in_prod() {
        let _g = EnvGuard::set(&[
            ("LOG_FORMAT", Some("pretty")),
            ("APP_ENV", Some("production")),
        ]);
        assert!(!should_use_json());
    }

    #[test]
    fn production_env_defaults_to_json() {
        let _g = EnvGuard::set(&[("LOG_FORMAT", None), ("APP_ENV", Some("production"))]);
        assert!(should_use_json());
    }

    #[test]
    fn non_production_env_defaults_to_pretty() {
        let _g = EnvGuard::set(&[("LOG_FORMAT", None), ("APP_ENV", Some("development"))]);
        assert!(!should_use_json());
    }

    #[test]
    fn unknown_log_format_falls_back_to_app_env() {
        let _g = EnvGuard::set(&[("LOG_FORMAT", Some("xml")), ("APP_ENV", Some("production"))]);
        assert!(should_use_json());
    }

    #[test]
    fn layer_is_constructable_in_both_modes() {
        // Smoke check that construction doesn't panic in either branch —
        // the returned trait object doesn't expose much to assert on, but
        // the function itself hitting both arms is the point.
        //
        // Drop the first guard before constructing the second so we don't
        // deadlock on the single-tenant env mutex. Two guards held in the
        // same scope would both try to acquire the same lock.
        {
            let _g = EnvGuard::set(&[("LOG_FORMAT", Some("json")), ("APP_ENV", None)]);
            let _: Box<dyn Layer<Registry> + Send + Sync + 'static> = layer();
        }
        {
            let _g = EnvGuard::set(&[("LOG_FORMAT", Some("pretty")), ("APP_ENV", None)]);
            let _: Box<dyn Layer<Registry> + Send + Sync + 'static> = layer();
        }
    }
}
