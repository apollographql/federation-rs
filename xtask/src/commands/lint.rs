use anyhow::Result;
use structopt::StructOpt;

use crate::tools::{CargoRunner, NpmRunner};

#[derive(Debug, StructOpt)]
pub(crate) struct Lint {}

impl Lint {
    pub(crate) fn run(&self) -> Result<()> {
        let cargo_runner = CargoRunner::new()?;
        cargo_runner.lint()?;
        let npm_runner = NpmRunner::new()?;
        npm_runner.lint()?;
        Ok(())
    }
}
