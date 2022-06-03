use anyhow::Result;
use structopt::StructOpt;

use crate::packages::PackageTag;
use crate::target::{Target, POSSIBLE_TARGETS};

use crate::tools::CargoRunner;

#[derive(Debug, StructOpt)]
pub(crate) struct Dist {
    /// The target to build for
    #[structopt(long = "target", env = "XTASK_TARGET", default_value, possible_values = &POSSIBLE_TARGETS)]
    pub(crate) target: Target,

    /// Package tag to build. Currently only the `composition` tag produces binaries.
    #[structopt(long, env = "CIRCLE_TAG")]
    pub(crate) package: Option<PackageTag>,

    /// Builds without the --release flag
    #[structopt(long)]
    pub(crate) debug: bool,
}

impl Dist {
    /// Builds binary crates
    pub(crate) fn run(&self, verbose: bool) -> Result<()> {
        let cargo_runner = CargoRunner::new(verbose)?;
        if let Some(package) = &self.package {
            let workspace_dir = package.get_workspace_dir()?;
            cargo_runner.build(&self.target, !self.debug, &workspace_dir)?;
        } else {
            cargo_runner.build_all(&self.target, !self.debug)?;
        }
        Ok(())
    }
}
