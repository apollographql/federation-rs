use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;
use lazy_static::lazy_static;

use std::{convert::TryFrom, env, process::Output, str};

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

lazy_static! {
    pub(crate) static ref PKG_PROJECT_ROOT: Utf8PathBuf =
        project_root().expect("Could not find the project root.");
}

#[macro_export]
macro_rules! info {
    ($msg:expr $(, $($tokens:tt)* )?) => {{
        let info_prefix = ansi_term::Colour::White.bold().paint("info:");
        eprintln!(concat!("{} ", $msg), &info_prefix $(, $($tokens)*)*);
    }};
}

pub(crate) fn get_workspace_roots() -> Result<Vec<Utf8PathBuf>> {
    let project_root = PKG_PROJECT_ROOT.clone();

    let package_directories = vec![
        project_root.join("federation-1"),
        project_root.join("federation-2"),
        project_root.join("federation-2").join("router-bridge"),
        project_root,
    ];

    let mut pkg_errs = Vec::new();
    for package_directory in &package_directories {
        if !package_directory.exists() {
            pkg_errs.push(format!("{package_directory} does not exist"));
        }
    }
    if let Some(first_pkg_err) = pkg_errs.pop() {
        let mut final_err = anyhow!(first_pkg_err);
        for pkg_err in pkg_errs {
            final_err = final_err.context(pkg_err);
        }
        Err(final_err)
    } else {
        Ok(package_directories)
    }
}

fn project_root() -> Result<Utf8PathBuf> {
    let manifest_dir = Utf8PathBuf::try_from(MANIFEST_DIR)
        .with_context(|| "Could not find the root directory.")?;
    let root_dir = manifest_dir
        .ancestors()
        .nth(1)
        .ok_or_else(|| anyhow!("Could not find project root."))?;
    Ok(root_dir.to_path_buf())
}

pub(crate) struct CommandOutput {
    pub(crate) stdout: String,
    pub(crate) _stderr: String,
    pub(crate) _output: Output,
}
