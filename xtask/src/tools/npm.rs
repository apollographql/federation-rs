use anyhow::{anyhow, Context, Result};
use which::which;

use std::str;

use crate::tools::Runner;

pub(crate) struct NpmRunner {
    runner: Runner,
}

impl NpmRunner {
    pub(crate) fn new() -> Result<Self> {
        Self::require_volta()?;
        let runner = Runner::new("npm");

        Ok(Self { runner })
    }

    pub(crate) fn lint(&self) -> Result<()> {
        self.npm_exec(&["install", "--prefix=harmonizer"])
            .context("Could not install all dependencies")?;
        self.npm_exec(&["run", "--prefix=harmonizer", "lint"])
            .context("Could not lint all packages")?;

        Ok(())
    }

    fn require_volta() -> Result<()> {
        which("volta")
            .map(|_| ())
            .map_err(|_| anyhow!("You must have `volta` installed."))
    }
    fn npm_exec(&self, args: &[&str]) -> Result<()> {
        self.runner.exec(args, None)?;
        Ok(())
    }
}
