use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;
use which::which;

use std::{collections::HashMap, str};

use crate::{
    tools::Runner,
    utils::{self, CommandOutput},
};

pub(crate) struct NpmRunner {
    runner: Runner,
    harmonizer_directories: HashMap<String, Utf8PathBuf>,
}

impl NpmRunner {
    pub(crate) fn new(verbose: bool) -> Result<Self> {
        Self::require_volta()?;
        let runner = Runner::new("npm", verbose)?;

        let harmonizer_directories = utils::get_harmonizer_crates()
            .context("Could not find one or more required packages")?;

        Ok(Self {
            runner,
            harmonizer_directories,
        })
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
        for (pkg_name, pkg_directory) in &self.harmonizer_directories {
            self.npm_exec(args, pkg_directory)
                .with_context(|| format!("Could not run command for `{}`", pkg_name))?;
        }
        Ok(())
    }

    fn npm_exec(&self, args: &[&str], directory: &Utf8PathBuf) -> Result<CommandOutput> {
        self.runner.exec(args, directory, None)
    }
}
