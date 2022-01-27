use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;
use which::which;

use std::{collections::HashMap, str};

use crate::{
    tools::Runner,
    utils::{CommandOutput, PKG_PROJECT_ROOT},
};

pub(crate) struct NpmRunner {
    runner: Runner,
    package_directories: HashMap<String, Utf8PathBuf>,
}

impl NpmRunner {
    pub(crate) fn new(verbose: bool) -> Result<Self> {
        let runner = Runner::new("npm", verbose)?;
        let project_root = PKG_PROJECT_ROOT.clone();

        let mut package_directories = HashMap::with_capacity(2);

        package_directories.insert(
            "harmonizer-0".to_string(),
            project_root.join("harmonizer-0"),
        );
        package_directories.insert(
            "harmonizer-2".to_string(),
            project_root.join("harmonizer-2"),
        );

        let mut pkg_errs = Vec::new();
        for (package_name, package_directory) in &package_directories {
            if !package_directory.exists() {
                pkg_errs.push(format!(
                    "{} does not exist at {}.",
                    package_name, package_directory
                ));
            }
        }
        if let Some(first_pkg_err) = pkg_errs.pop() {
            let mut final_err = anyhow!(first_pkg_err);
            for pkg_err in pkg_errs {
                final_err = final_err.context(pkg_err);
            }
            Err(final_err.context("Could not find one or more required npm packages."))
        } else {
            Ok(Self {
                runner,
                package_directories,
            })
        }
    }

    pub(crate) fn lint(&self) -> Result<()> {
        self.require_volta()?;
        self.run_all(&["install"])
            .context("Could not install all dependencies")?;
        self.run_all(&["run", "lint"])
            .context("Could not lint all packages")?;

        Ok(())
    }

    fn require_volta(&self) -> Result<()> {
        which("volta")
            .map(|_| ())
            .map_err(|_| anyhow!("You must have `volta` installed to run this command."))
    }

    fn run_all(&self, args: &[&str]) -> Result<()> {
        for (pkg_name, pkg_directory) in &self.package_directories {
            self.npm_exec(args, pkg_directory)
                .with_context(|| format!("Could not run command for `{}`", pkg_name))?;
        }
        Ok(())
    }

    fn npm_exec(&self, args: &[&str], directory: &Utf8PathBuf) -> Result<CommandOutput> {
        self.runner.exec(args, directory, None)
    }
}
