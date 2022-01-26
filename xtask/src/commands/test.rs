use anyhow::Result;
use structopt::StructOpt;

use crate::tools::CargoRunner;

#[derive(Debug, StructOpt)]
pub struct Test {}

impl Test {
    pub fn run(&self, verbose: bool) -> Result<()> {
        let cargo_runner = CargoRunner::new(verbose)?;
        cargo_runner.test()
    }
}
