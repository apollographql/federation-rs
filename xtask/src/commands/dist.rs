use anyhow::Result;
use structopt::StructOpt;

use crate::tools::CargoRunner;

#[derive(Debug, StructOpt)]
pub struct Dist {}

impl Dist {
    pub fn run(&self, verbose: bool) -> Result<()> {
        let cargo_runner = CargoRunner::new(verbose)?;
        cargo_runner.build()?;
        Ok(())
    }
}
