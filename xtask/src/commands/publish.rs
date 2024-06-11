use anyhow::{Context, Result};
use structopt::StructOpt;

use crate::tools::{CargoRunner, GitRunner};

#[derive(Debug, StructOpt)]
pub(crate) struct Publish {}

impl Publish {
    pub fn run(&self) -> Result<()> {
        let git_runner = GitRunner::new()?;

        // Check if (git) HEAD is pointing to a package tag
        //
        // NOTE: typically this will only succeed after running `cargo xtask tag --package {package} --real-publish`
        //       cargo xtask publish is executed by CircleCI
        let package_tag = git_runner
            .get_package_tag()
            .context("There are no valid package tags pointing to HEAD.")?;

        // currently all packages have a library so just publish them.
        // if we ever wanted to publish just a binary without a library
        // to accompany it, change the function signature to
        // PackageGroup::get_library(&self) -> Option<LibraryCrate>
        // and handle it here.

        let cargo_runner = CargoRunner::new()?;
        cargo_runner.publish(&package_tag.package_group.get_library())?;

        Ok(())
    }
}
