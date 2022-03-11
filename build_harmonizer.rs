use deno_core::{JsRuntime, RuntimeOptions};
use semver::Version;
use serde_json::Value as JsonValue;
use std::{env, fs, path::Path, process::Command, str};
use toml_edit::{value as new_toml_value, Document as TomlDocument};

// this build.rs file is used by both `harmonizer-0` and `harmonizer-2`
// to keep the crate version in line with the appropriate npm package
// and to maintain a Cargo.publish.toml that is only used for publishes

fn main() {
    // Always rerun the script
    let target_dir = std::env::var_os("OUT_DIR").unwrap();
    println!("cargo:rerun-if-changed={:?}", target_dir);

    if cfg!(target_arch = "musl") {
        panic!("This package cannot be built for musl architectures.");
    }

    update_manifests();
    bundle_for_deno();
}

// runs `npm ci` && `npm run build` in the current `harmonizer-x` workspace crate
fn bundle_for_deno() {
    let npm = which::which("npm").expect("You must have npm installed to build this crate.");
    let current_dir = std::env::current_dir().unwrap();

    println!(
        "cargo:warning=running `npm ci` in {}",
        &current_dir.display()
    );
    assert!(Command::new(&npm)
        .current_dir(&current_dir)
        .args(&["ci"])
        .status()
        .expect("Could not get status of `npm ci`")
        .success());

    println!(
        "cargo:warning=running `npm run format` in {}",
        &current_dir.display()
    );
    assert!(Command::new(&npm)
        .current_dir(&current_dir)
        .args(&["run", "format"])
        .status()
        .expect("Could not get status of `npm run format`")
        .success());

    println!(
        "cargo:warning=running `npm run build` in {}",
        &current_dir.display()
    );
    assert!(Command::new(&npm)
        .current_dir(&current_dir)
        .args(&["run", "build"])
        .status()
        .expect("Could not get status of `npm run build`")
        .success());
}

// updates `Cargo.toml`, `Cargo.publish.toml`, and `package.json` in the current `harmonizer-x` workspace crate
// in addition to the `Cargo.publish.toml` in the workspace
fn update_manifests() {
    let current_dir = std::env::current_dir().expect("Could not find the current directory.");
    let build_manifest_path = current_dir.join("Cargo.toml");
    let publish_manifest_path = current_dir.join("Cargo.publish.toml");
    let harmonizer_version = update_this_manifest(&build_manifest_path, &publish_manifest_path);

    let workspace_dir = current_dir
        .parent()
        .expect("Could not find parent directory.");
    let build_workspace_manifest_path = workspace_dir.join("Cargo.toml");
    let publish_workspace_manifest_path = workspace_dir.join("Cargo.publish.toml");
    update_workspace_manifest(
        &build_workspace_manifest_path,
        &publish_workspace_manifest_path,
    );
    let supergraph_version = current_dir
        .file_stem()
        .expect("Could not find file stem of current directory.")
        .to_string_lossy()
        .to_string()
        .split('-')
        .collect::<Vec<&str>>()[1]
        .to_string();
    let supergraph_dir = workspace_dir.join(format!("supergraph-{}", &supergraph_version));
    let build_supergraph_manifest_path = supergraph_dir.join("Cargo.toml");
    let publish_supergraph_manifest_path = supergraph_dir.join("Cargo.publish.toml");
    update_supergraph_manifest(
        &build_supergraph_manifest_path,
        &publish_supergraph_manifest_path,
        &harmonizer_version,
    );
}

// Updates the `Cargo.toml` and `Cargo.publish.toml` for this version of harmonizer
// and returns the version as a string.
fn update_this_manifest(build_manifest_path: &Path, publish_manifest_path: &Path) -> Version {
    let build_manifest_contents =
        fs::read_to_string(&build_manifest_path).expect("Could not read 'Cargo.toml'");
    let mut build_manifest = build_manifest_contents
        .parse::<TomlDocument>()
        .expect("Cargo.toml is not valid TOML");

    let build_manifest_name = build_manifest["package"]["name"]
        .as_str()
        .expect("`package.name` in Cargo.toml is not a string");

    let js_composition_version = match build_manifest_name {
        "harmonizer-0" => get_npm_dep_version("@apollo/federation"),
        "harmonizer-2" => get_npm_dep_version("@apollo/composition"),
        _ => panic!("attempting to build unknown crate"),
    };

    let crate_version = Version::parse(
        build_manifest["package"]["version"]
            .as_str()
            .expect("`package.version` in Cargo.toml is not a string"),
    )
    .expect("Crate version is not valid semver");

    if js_composition_version != crate_version {
        build_manifest["package"]["version"] = new_toml_value(js_composition_version.to_string());
        fs::write(&build_manifest_path, build_manifest.to_string())
            .expect("Could not write updated Cargo.toml");
    }

    build_manifest["package"]["publish"] = new_toml_value(true);
    build_manifest["package"]["name"] = new_toml_value("harmonizer");
    build_manifest["package"]
        .as_table_mut()
        .expect("`package` is not a table in Cargo.toml")
        .remove("build")
        .expect("Could not remove `package.build` from Cargo.toml");

    fs::write(
        &publish_manifest_path,
        prepend_autogen_warning(&build_manifest.to_string()),
    )
    .expect("Could not write updated Cargo.publish.toml");

    js_composition_version
}

fn update_workspace_manifest(
    build_workspace_manifest_path: &Path,
    publish_workspace_manifest_path: &Path,
) {
    let workspace_manifest_contents =
        fs::read_to_string(&build_workspace_manifest_path).expect("Could not read 'Cargo.toml'");
    let mut workspace_manifest = workspace_manifest_contents
        .parse::<TomlDocument>()
        .expect("Cargo.toml is not valid TOML");
    let workspace_members = workspace_manifest["workspace"]["members"]
        .as_array_mut()
        .expect("`workspace.members` is not an array in Cargo.toml");

    let mut i = 0;
    while i < workspace_members.len() {
        let current_member = workspace_members
            .get(i)
            .unwrap()
            .as_str()
            .expect("`workspace.members` in Cargo.toml contains values that are not strings");
        if current_member.contains("harmonizer") || current_member.contains("supergraph") {
            workspace_members.remove(i);
        } else {
            i += 1;
        }
    }
    workspace_members.push("harmonizer");
    workspace_members.push("supergraph");
    fs::write(
        &publish_workspace_manifest_path,
        prepend_autogen_warning(&workspace_manifest.to_string()),
    )
    .expect("Could not write updated Cargo.publish.toml");
}

fn update_supergraph_manifest(
    build_supergraph_manifest_path: &Path,
    publish_supergraph_manifest_path: &Path,
    new_package_version: &Version,
) {
    let supergraph_manifest_contents =
        fs::read_to_string(&build_supergraph_manifest_path).expect("Could not read Cargo.toml");
    let mut supergraph_manifest = supergraph_manifest_contents
        .parse::<TomlDocument>()
        .expect("Cargo.toml is not valid TOML");
    supergraph_manifest["package"]["version"] = new_toml_value(new_package_version.to_string());
    fs::write(
        build_supergraph_manifest_path,
        supergraph_manifest.to_string(),
    )
    .expect("Could not update Cargo.toml");
    supergraph_manifest["package"]["name"] = new_toml_value("supergraph");
    supergraph_manifest["dependencies"]["harmonizer"]["path"] = new_toml_value("../harmonizer");
    supergraph_manifest["dependencies"]["harmonizer"]["package"] = new_toml_value("harmonizer");

    fs::write(
        publish_supergraph_manifest_path,
        prepend_autogen_warning(&supergraph_manifest.to_string()),
    )
    .expect("Could not write updated Cargo.publish.toml");
}

// parses the output of the current `package.json` to get the version of an npm dependency
fn get_npm_dep_version(dep_name: &str) -> Version {
    let current_dir = env::current_dir().unwrap();
    let npm_manifest_path = current_dir.join("package.json");
    let mut npm_manifest_contents: JsonValue = serde_json::from_str(
        &fs::read_to_string(&npm_manifest_path).expect("Could not read package.json"),
    )
    .expect("package.json is not valid JSON");
    let version_string = npm_manifest_contents["dependencies"][dep_name]
        .as_str()
        .unwrap_or_else(|| panic!("`.dependencies.{}` is not a string", dep_name));
    let parsed_version = Version::parse(version_string).unwrap_or_else(|_| {
        panic!(
            "version for `{}`, `{}`, is not valid semver",
            dep_name, version_string
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

fn prepend_autogen_warning(manifest_contents: &str) -> String {
    format!(
            "#### ⚠️ DO NOT EDIT THIS FILE ⚠️ ####\n## it is autogenerated in build_harmonizer.rs ##\n{}\n",
            manifest_contents,
        )
}
