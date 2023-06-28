use anyhow::Result;
use structopt::StructOpt;

use crate::packages::PackageTag;
use crate::tools::{CargoRunner, NpmRunner};

#[derive(Debug, StructOpt)]
pub(crate) struct Lint {
    /// Package tag to build. Currently only the `composition` tag produces binaries.
    #[structopt(long, env = "CIRCLE_TAG")]
    pub(crate) package: Option<PackageTag>,
}

impl Lint {
    pub(crate) fn run(&self) -> Result<()> {
        let cargo_runner = CargoRunner::new()?;
        if let Some(package) = &self.package {
            cargo_runner.lint(&package.get_workspace_dir()?)?;
        } else {
            cargo_runner.lint_all()?;
        }
        let npm_runner = NpmRunner::new()?;
        npm_runner.lint()?;
        Ok(())
    }
}
