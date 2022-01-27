use std::process::Command;

fn main() {
    let target_dir = std::env::var_os("OUT_DIR").unwrap();
    // Always rerun the script
    println!("cargo:rerun-if-changed={:?}", target_dir);
    if std::env::var_os("SKIP_JS_BUNDLE").is_none() {
        bundle_for_deno();
    } else {
        println!("cargo:warning=$SKIP_JS_BUNDLE is set, skipping bundle step");
    }
}

fn bundle_for_deno() {
    // $SKIP_JS_BUNDLE is set in our CircleCI builds after the bundler is run exactly once
    // and persisted to the workspace.
    //
    // You may set this yourself if you find that the build steps are taking too long.
    if std::env::var_os("SKIP_JS_BUNDLE").is_none() {
        let npm = which::which("npm").unwrap();
        let current_dir = std::env::current_dir().unwrap();

        println!(
            "cargo:warning=running `npm install` in {}",
            &current_dir.display()
        );
        assert!(Command::new(&npm)
            .current_dir(&current_dir)
            .args(&["install"])
            .status()
            .unwrap()
            .success());

        println!(
            "cargo:warning=running `npm run build` in {}",
            &current_dir.display()
        );
        assert!(Command::new(&npm)
            .current_dir(&current_dir)
            .args(&["run", "build"])
            .status()
            .unwrap()
            .success());
    } else {
        println!("cargo:warning=$SKIP_JS_BUNDLE is set, skipping bundle step");
    }
}
