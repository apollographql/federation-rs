use anyhow::Result;
use structopt::StructOpt;

use crate::packages::PackageTag;
use crate::tools::CargoRunner;

#[derive(Debug, StructOpt)]
pub(crate) struct Test {
    /// Package tag to build. Currently only the `composition` tag produces binaries.
    #[structopt(long, env = "CIRCLE_TAG")]
    pub(crate) package: Option<PackageTag>,
}

impl Test {
    /// Tests crates
    pub(crate) fn run(&self, verbose: bool) -> Result<()> {
        let cargo_runner = CargoRunner::new(verbose)?;
        if let Some(package) = &self.package {
            let workspace_dir = package.get_workspace_dir()?;
            cargo_runner.test(&workspace_dir)?;
        } else {
            cargo_runner.test_all()?;
        }
        Ok(())
    }
}
