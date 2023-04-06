use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // only do `npm` related stuff if we're _not_ publishing to crates.io
    if std::fs::metadata("./package.json").is_ok() {
        println!("cargo:rerun-if-changed=js-src");

        let current_dir = std::env::current_dir().unwrap();

        update_bridge(&current_dir);
    } else {
        // the crate has been published, no need to rerun
        println!("cargo:rerun-if-changed=build.rs");
    }

    let out_dir: PathBuf = std::env::var_os("OUT_DIR")
        .expect("$OUT_DIR not set.")
        .into();

    create_snapshot(&out_dir);
}

fn update_bridge(current_dir: &Path) {
    println!("cargo:warning=Updating router-bridge");
    let npm = which::which("npm").expect("'npm' is not available");

    if cfg!(debug_assertions) {
        // in debug mode we want to update the package-lock.json
        // so we run `npm install`
        println!("cargo:warning=running `npm install`");
        assert!(Command::new(&npm)
            .current_dir(current_dir)
            .args(["install"])
            .status()
            .unwrap()
            .success());
    } else {
        // in release mode, we're probably running in CI
        // and want the version we publish to match
        // the git source
        // so we run `npm ci`.
        println!("cargo:warning=running `npm ci`");
        assert!(Command::new(&npm)
            .current_dir(current_dir)
            .args(["ci"])
            .status()
            .unwrap()
            .success());
    }

    println!("cargo:warning=running `npm run format`");
    assert!(Command::new(&npm)
        .current_dir(current_dir)
        .args(["run", "format"])
        .status()
        .unwrap()
        .success());

    println!("cargo:warning=running `npm run build`");
    assert!(Command::new(&npm)
        .current_dir(current_dir)
        .args(["run", "build"])
        .status()
        .unwrap()
        .success());
}

#[cfg(feature = "docs_rs")]
fn create_snapshot(out_dir: &Path) {
    // If we're building on docs.rs we just create
    // an empty snapshot file and return, because `rusty_v8`
    // doesn't actually compile on docs.rs
    std::fs::write(out_dir.join("query_runtime.snap"), []).unwrap();
}

#[cfg(not(feature = "docs_rs"))]
fn create_snapshot(out_dir: &Path) {
    use deno_core::{JsRuntime, RuntimeOptions};
    use std::fs::{read_to_string, File};
    use std::io::Write;

    let options = RuntimeOptions {
        will_snapshot: true,
        extensions: vec![
            deno_webidl::init(),
            deno_console::init(),
            deno_url::init(),
            deno_web::init::<Permissions>(deno_web::BlobStore::default(), Default::default()),
            deno_crypto::init(None),
        ],
        ..Default::default()
    };
    let mut runtime = JsRuntime::new(options);

    // The runtime automatically contains a Deno.core object with several
    // functions for interacting with it.
    let runtime_str = read_to_string("bundled/runtime.js").unwrap();
    runtime
        .execute_script("<init>", &runtime_str)
        .expect("unable to initialize router bridge runtime environment");

    // Load the composition library.
    let bridge_str = read_to_string("bundled/bridge.js").unwrap();
    runtime
        .execute_script("bridge.js", &bridge_str)
        .expect("unable to evaluate bridge module");

    // Create our base query snapshot which will be included in
    // src/js.rs to initialise our JsRuntime().
    let mut snap = File::create(out_dir.join("query_runtime.snap")).unwrap();
    snap.write_all(&runtime.snapshot()).unwrap();
}

#[derive(Clone)]
struct Permissions;

impl deno_web::TimersPermission for Permissions {
    fn allow_hrtime(&mut self) -> bool {
        unreachable!("snapshotting!")
    }

    fn check_unstable(&self, _state: &deno_core::OpState, _api_name: &'static str) {
        unreachable!("snapshotting!")
    }
}
