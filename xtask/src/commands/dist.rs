use anyhow::Result;
use structopt::StructOpt;

use crate::target::{Target, POSSIBLE_TARGETS};

use crate::tools::CargoRunner;

#[derive(Debug, StructOpt)]
pub(crate) struct Dist {
    /// The target to build for
    #[structopt(long = "target", env = "XTASK_TARGET", default_value, possible_values = &POSSIBLE_TARGETS)]
    pub(crate) target: Target,

    /// Builds without the --release flag
    #[structopt(long)]
    pub(crate) debug: bool,
}

impl Dist {
    /// Builds binary crates
    pub(crate) fn run(&self) -> Result<()> {
        let cargo_runner = CargoRunner::new()?;
        cargo_runner.build(&self.target, !self.debug)?;
        Ok(())
    }
}
