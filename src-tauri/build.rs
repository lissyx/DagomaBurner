use std::env;
use std::path::Path;

/// Builds the frontend that non-dev builds embed. Node.js and the npm
/// dependencies are installed under target/ (see tools/frontend).
fn build_frontend() {
    for path in [
        "../src",
        "../package.json",
        "../package-lock.json",
        "../vite.config.js",
        "../scripts/frontend.mjs",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    println!("cargo:rerun-if-env-changed=DAGOMAPP_NODE");

    // `cargo tauri dev` serves the frontend from the Vite dev server instead.
    if tauri_build::is_dev() {
        return;
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut command = dagomapp_frontend::command(&root, "build")
        .unwrap_or_else(|e| panic!("can't build the frontend: {e}"));
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        command.env("TAURI_ENV_PLATFORM", "windows");
    }
    if env::var("PROFILE").as_deref() == Ok("debug") {
        command.env("TAURI_ENV_DEBUG", "true");
    }

    let status = command
        .status()
        .unwrap_or_else(|e| panic!("can't run Node.js to build the frontend: {e}"));
    assert!(status.success(), "building the frontend failed");
}

fn main() {
    build_frontend();
    tauri_build::build()
}
