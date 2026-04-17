//! One-shot generator that writes the OpenAPI JSON snapshot. Invoked by the
//! snapshot test flow via `cargo run --example openapi_gen -- <path>`.
//!
//! Run from `backend/`:
//! ```bash
//! cargo run --example openapi_gen -- tests/snapshots/openapi.json
//! ```

use std::path::PathBuf;

use swings_api::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("tests/snapshots/openapi.json"));

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("mkdir snapshot parent");
    }

    let doc = ApiDoc::openapi();
    let mut out = serde_json::to_string_pretty(&doc).expect("openapi JSON serialization");
    out.push('\n');
    std::fs::write(&path, &out).expect("write snapshot");
    eprintln!("Wrote {}", path.display());
}
