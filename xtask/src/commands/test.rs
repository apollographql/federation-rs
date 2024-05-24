use anyhow::Result;
use structopt::StructOpt;

use crate::tools::CargoRunner;

#[derive(Debug, StructOpt)]
pub(crate) struct Test {}

impl Test {
    /// Tests crates
    pub(crate) fn run(&self) -> Result<()> {
        CargoRunner::new()?.test()
    }
}
