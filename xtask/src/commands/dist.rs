use anyhow::{anyhow, Result};
use camino::Utf8PathBuf;
use structopt::StructOpt;

use crate::packages::PackageTag;
use crate::target::{Target, POSSIBLE_TARGETS};
use crate::{commands, tools::CargoRunner};

#[derive(Debug, StructOpt)]
pub(crate) struct Dist {
    /// The target to build for
    #[structopt(long = "target", env = "XTASK_TARGET", default_value, possible_values = &POSSIBLE_TARGETS)]
    pub(crate) target: Target,

    /// Package tag to build. Currently only the `composition` tag produces binaries.
    #[structopt(long)]
    pub(crate) package: PackageTag,
}

impl Dist {
    /// Builds binary crates and returns the path to the workspace it was built from
    pub(crate) fn run(&self, verbose: bool) -> Result<Utf8PathBuf> {
        let stage_env = commands::Prep {
            target: self.target.clone(),
            package: self.package.clone(),
        }
        .run(verbose)?;

        if let Some(stage_env) = stage_env {
            let mut cargo_runner = CargoRunner::new_with_path(verbose, &stage_env.stage_dir)?;
            cargo_runner.build(&self.target, true)?;
            Ok(stage_env.stage_dir)
        } else {
            Err(anyhow!(
                "expected stage environment to exist for this package group"
            ))
        }
    }
}
