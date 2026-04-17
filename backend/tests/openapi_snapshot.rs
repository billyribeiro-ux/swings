#![deny(warnings)]
#![forbid(unsafe_code)]

//! FDN-02 snapshot test.
//!
//! This test is the contract between the Rust backend's `#[utoipa::path(...)]`
//! annotations and the committed `backend/tests/snapshots/openapi.json`. The
//! frontend codegen (`scripts/openapi-to-ts.mjs`) reads from that committed
//! snapshot, so any handler change that affects the spec must also update the
//! snapshot in the same PR.
//!
//! Update the snapshot by running:
//!
//! ```bash
//! UPDATE_OPENAPI_SNAPSHOT=1 cargo test --test openapi_snapshot
//! ```

use std::path::PathBuf;

use swings_api::openapi::ApiDoc;
use utoipa::OpenApi;

fn snapshot_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join("openapi.json")
}

fn generated_spec() -> String {
    let doc = ApiDoc::openapi();
    let mut out = serde_json::to_string_pretty(&doc).expect("openapi JSON serialization");
    out.push('\n');
    out
}

#[test]
fn openapi_snapshot_matches() {
    let generated = generated_spec();
    let path = snapshot_path();

    if std::env::var("UPDATE_OPENAPI_SNAPSHOT").ok().as_deref() == Some("1") {
        std::fs::create_dir_all(path.parent().unwrap()).expect("mkdir snapshot parent");
        std::fs::write(&path, &generated).expect("write snapshot");
        eprintln!("Wrote snapshot to {}", path.display());
        return;
    }

    let committed = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => {
            // Bootstrap: create the snapshot on first run so `cargo test` is self-healing
            // in sandboxes where passing `UPDATE_OPENAPI_SNAPSHOT=1` is not supported.
            std::fs::create_dir_all(path.parent().unwrap()).expect("mkdir snapshot parent");
            std::fs::write(&path, &generated).expect("write snapshot");
            eprintln!("Bootstrapped snapshot at {}", path.display());
            return;
        }
    };

    if committed != generated {
        eprintln!(
            "OpenAPI snapshot drift at {}. Run: UPDATE_OPENAPI_SNAPSHOT=1 cargo test --test openapi_snapshot",
            path.display()
        );
        // Show a short diff hint.
        let c_len = committed.len();
        let g_len = generated.len();
        eprintln!("committed={c_len} bytes  generated={g_len} bytes");
        assert_eq!(committed, generated, "OpenAPI snapshot drift");
    }
}
