use anyhow::Result;
use structopt::StructOpt;

use crate::{
    packages::PackageTag,
    target::Target,
    tools::{CargoRunner, GitRunner},
};

#[derive(Debug, StructOpt)]
pub(crate) struct Tag {
    /// this command does a dry run tag by default,
    /// to really run it, pass --real-publish
    #[structopt(long)]
    pub(crate) real_publish: bool,

    #[structopt(long)]
    pub(crate) package: PackageTag,
}

impl Tag {
    pub(crate) fn run(&self, verbose: bool) -> Result<()> {
        let git_runner = GitRunner::new(verbose)?;
        git_runner.can_tag(&self.package)?;
        let mut cargo_runner = CargoRunner::new(verbose)?;
        cargo_runner.test()?;
        cargo_runner.lint()?;
        cargo_runner.build(&Target::Other, false)?;
        git_runner.tag_release(&self.package, !self.real_publish)
    }
}
