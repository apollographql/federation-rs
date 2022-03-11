use deno_core::{JsRuntime, RuntimeOptions};
use semver::Version;
use serde_json::Value as JsonValue;
use std::error::Error;
use std::fs::{read_to_string, File};
use std::io::Write;
use std::{env, fs, path::Path, process::Command, str};
use toml_edit::{value as new_toml_value, Document as TomlDocument};

// this build.rs file is used by `harmonizer` to generate the right Deno snapshots
fn main() {
    println!("cargo:warning=generating deno snapshots");
    create_snapshot().expect("unable to create v8 snapshot: query_runtime.snap");
}

fn create_snapshot() -> Result<(), Box<dyn Error>> {
    let options = RuntimeOptions {
        will_snapshot: true,
        ..Default::default()
    };
    let mut runtime = JsRuntime::new(options);

    // The runtime automatically contains a Deno.core object with several
    // functions for interacting with it.
    let runtime_source = read_to_string("deno/runtime.js")?;
    runtime
        .execute_script("<init>", &runtime_source)
        .expect("unable to initialize harmonizer runtime environment");

    // Load the composition library.
    let composition_source = read_to_string("dist/composition.js")?;
    runtime
        .execute_script("composition.js", &composition_source)
        .expect("unable to evaluate composition module");

    // Create our base query snapshot which will be included in
    // src/js.rs to initialise our JsRuntime().
    let mut snap = File::create("snapshots/query_runtime.snap")?;
    snap.write_all(&runtime.snapshot())?;

    Ok(())
}
