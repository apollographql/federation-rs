use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;
use which::which;

use std::{path::Path, str};

use crate::{
    tools::Runner,
    utils::{self, CommandOutput},
};

pub(crate) struct NpmRunner {
    runner: Runner,
    npm_roots: Vec<Utf8PathBuf>,
}

impl NpmRunner {
    pub(crate) fn new(verbose: bool) -> Result<Self> {
        Self::require_volta()?;
        let runner = Runner::new("npm", verbose)?;

        let workspace_roots =
            utils::get_workspace_roots().context("Could not find one or more required packages")?;

        let mut npm_roots = Vec::with_capacity(workspace_roots.len());
        for workspace_root in workspace_roots {
            let maybe_harmonizer = workspace_root.join("harmonizer");
            if Path::new(&maybe_harmonizer).exists() {
                npm_roots.push(maybe_harmonizer);
            }
        }

        Ok(Self { runner, npm_roots })
    }

    pub(crate) fn lint(&self) -> Result<()> {
        self.run_all(&["install"])
            .context("Could not install all dependencies")?;
        self.run_all(&["run", "lint"])
            .context("Could not lint all packages")?;

        Ok(())
    }

    fn require_volta() -> Result<()> {
        which("volta")
            .map(|_| ())
            .map_err(|_| anyhow!("You must have `volta` installed."))
    }

    fn run_all(&self, args: &[&str]) -> Result<()> {
        for pkg_directory in &self.npm_roots {
            self.npm_exec(args, pkg_directory)
                .with_context(|| format!("Could not run command in `{pkg_directory}`"))?;
        }
        Ok(())
    }

    fn npm_exec(&self, args: &[&str], directory: &Utf8PathBuf) -> Result<CommandOutput> {
        self.runner.exec(args, directory, None)
    }
}
