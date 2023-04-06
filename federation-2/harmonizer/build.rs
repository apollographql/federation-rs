use deno_core::{JsRuntime, RuntimeOptions};
use semver::Version;
use serde_json::Value as JsonValue;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::{env, error::Error, fs, io::Write, path::Path, process::Command};
use toml_edit::{value as new_toml_value, Document as TomlDocument};

// this build.rs file is used by both `federation-1/harmonizer` and `federation-2/harmonizer`
// to keep the crate version in line with the appropriate npm package
// and to build the V8 snapshots

fn main() {
    // Always rerun the script
    let out_dir = std::env::var_os("OUT_DIR").expect("$OUT_DIR not set.");
    println!("cargo:rerun-if-changed={:?}", &out_dir);
    let out_dir: PathBuf = out_dir.into();
    if cfg!(target_arch = "musl") {
        panic!("This package cannot be built for musl architectures.");
    }

    let current_dir = std::env::current_dir().unwrap();

    // only do `npm` related stuff if we're _not_ publishing to crates.io
    // package.json is not in the `includes` section of `Cargo.toml`
    if std::fs::metadata("./package.json").is_ok() {
        update_manifests();
        bundle_for_deno(&current_dir);
    }

    // always create the snapshot
    create_snapshot(&out_dir).expect("unable to create v8 snapshot: composition.snap");
}

// runs `npm install` && `npm run build` in the current `harmonizer-x` workspace crate
fn bundle_for_deno(current_dir: &Path) {
    let npm = which::which("npm").expect("You must have npm installed to build this crate.");

    if cfg!(debug_assertions) {
        // in debug mode we want to update the package-lock.json
        // so we run `npm install`
        println!(
            "cargo:warning=running `npm install` in {}",
            &current_dir.display()
        );
        assert!(Command::new(&npm)
            .current_dir(current_dir)
            .args(["install"])
            .status()
            .expect("Could not get status of `npm install`")
            .success());
    } else {
        // in release mode, we're probably running in CI
        // and want the version we publish to match
        // the git source
        // so we run `npm ci`.
        println!(
            "cargo:warning=running `npm ci` in {}",
            &current_dir.display()
        );
        assert!(Command::new(&npm)
            .current_dir(current_dir)
            .args(["ci"])
            .status()
            .expect("Could not get status of `npm ci`")
            .success());
    }

    println!(
        "cargo:warning=running `npm run format` in {}",
        &current_dir.display()
    );
    assert!(Command::new(&npm)
        .current_dir(current_dir)
        .args(["run", "format"])
        .status()
        .expect("Could not get status of `npm run format`")
        .success());

    println!(
        "cargo:warning=running `npm run build` in {}",
        &current_dir.display()
    );
    assert!(Command::new(&npm)
        .current_dir(current_dir)
        .args(["run", "build"])
        .status()
        .expect("Could not get status of `npm run build`")
        .success());
}

// updates `Cargo.toml` and `package.json` in the current `federation-x/harmonizer` crate
fn update_manifests() {
    let current_dir = std::env::current_dir().expect("Could not find the current directory.");
    let harmonizer_manifest_path = current_dir.join("Cargo.toml");
    let maybe_harmonizer_version = update_this_manifest(&harmonizer_manifest_path);
    if let Some(harmonizer_version) = maybe_harmonizer_version {
        println!(
            "cargo:warning=updated {} to {}",
            &harmonizer_manifest_path.display(),
            &harmonizer_version
        );
        let federation_workspace_dir = current_dir
            .parent()
            .expect("Could not find parent directory.");
        let supergraph_dir = federation_workspace_dir.join("supergraph");
        let supergraph_manifest_path = supergraph_dir.join("Cargo.toml");
        update_supergraph_manifest(&supergraph_manifest_path, &harmonizer_version);
    }
}

// Updates the `Cargo.toml` for this version of harmonizer
// and returns Some(Version) if it was updated and None if it was not
fn update_this_manifest(build_manifest_path: &Path) -> Option<Version> {
    let build_manifest_contents =
        fs::read_to_string(build_manifest_path).expect("Could not read 'Cargo.toml'");
    let mut build_manifest = build_manifest_contents
        .parse::<TomlDocument>()
        .expect("Cargo.toml is not valid TOML");

    let js_composition_version = get_underlying_composition_npm_module_version();

    let crate_version = Version::parse(
        build_manifest["package"]["version"]
            .as_str()
            .expect("`package.version` in Cargo.toml is not a string"),
    )
    .expect("Crate version is not valid semver");

    if js_composition_version != crate_version {
        build_manifest["package"]["version"] = new_toml_value(js_composition_version.to_string());
        fs::write(build_manifest_path, build_manifest.to_string())
            .expect("Could not write updated Cargo.toml");
        Some(js_composition_version)
    } else {
        None
    }
}

fn update_supergraph_manifest(supergraph_manifest_path: &Path, new_package_version: &Version) {
    let supergraph_manifest_contents =
        fs::read_to_string(supergraph_manifest_path).expect("Could not read Cargo.toml");
    let mut supergraph_manifest = supergraph_manifest_contents
        .parse::<TomlDocument>()
        .expect("Cargo.toml is not valid TOML");
    supergraph_manifest["package"]["version"] = new_toml_value(new_package_version.to_string());
    fs::write(supergraph_manifest_path, supergraph_manifest.to_string())
        .expect("Could not update Cargo.toml");
}

// reads package.json, finds the correct composition JS dependency, and returns its version
fn get_underlying_composition_npm_module_version() -> Version {
    let current_dir = env::current_dir().unwrap();
    let npm_manifest_path = current_dir.join("package.json");
    let mut npm_manifest_contents: JsonValue = serde_json::from_str(
        &fs::read_to_string(&npm_manifest_path).expect("Could not read package.json"),
    )
    .expect("package.json is not valid JSON");

    let maybe_federation = npm_manifest_contents["dependencies"]["@apollo/federation"].as_str();
    let maybe_composition = npm_manifest_contents["dependencies"]["@apollo/composition"].as_str();
    let (dep_name, version_string) = match (maybe_federation, maybe_composition) {
        (None, Some(composition)) => {
            let dep_name = "@apollo/composition".to_string();
            let version_str = npm_manifest_contents["dependencies"][&dep_name]
                .as_str()
                .unwrap_or_else(|| panic!("`.dependencies.{}` is not a string", &composition));
            (dep_name, version_str.to_string())
        }
        (Some(federation), None) => {
            let dep_name = "@apollo/federation".to_string();
            let version_str = npm_manifest_contents["dependencies"][&dep_name]
                .as_str()
                .unwrap_or_else(|| panic!("`.dependencies.{}` is not a string", &federation));
            (dep_name, version_str.to_string())
        }
        (Some(_federation), Some(_composition)) => unreachable!(
            "Found both `@apollo/federation` and `@apollo/composition`. There should only be one."
        ),
        (None, None) => unreachable!(
            "Underlying npm module must be either `@apollo/federation` or `@apollo/composition`"
        ),
    };

    let parsed_version = Version::parse(&version_string).unwrap_or_else(|_| {
        panic!(
            "version for `{}`, `{}`, is not valid semver",
            &dep_name, &version_string
        )
    });

    npm_manifest_contents["version"] = JsonValue::from(version_string);
    fs::write(
        &npm_manifest_path,
        serde_json::to_string_pretty(&npm_manifest_contents).expect("Could not pretty print JSON"),
    )
    .expect("Could not write updated contents to package.json");

    parsed_version
}

fn create_snapshot(out_dir: &Path) -> Result<(), Box<dyn Error>> {
    let options = RuntimeOptions {
        will_snapshot: true,
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
    let bridge_str = read_to_string("bundled/composition_bridge.js").unwrap();
    runtime
        .execute_script("composition_bridge.js", &bridge_str)
        .expect("unable to evaluate bridge module");

    // Create our base query snapshot which will be included in
    // src/js.rs to initialise our JsRuntime().
    println!("cargo:warning={:?}", &out_dir);
    let mut snap = fs::File::create(out_dir.join("composition.snap"))?;
    snap.write_all(&runtime.snapshot())?;

    Ok(())
}
