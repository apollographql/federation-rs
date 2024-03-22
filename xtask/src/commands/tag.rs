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

    #[structopt(long, env = "CIRCLE_TAG")]
    pub(crate) package: PackageTag,
}

impl Tag {
    pub(crate) fn run(&self) -> Result<()> {
        let git_runner = GitRunner::new()?;
        let allow_non_main = !self.package.version.pre.is_empty();
        git_runner.can_tag(allow_non_main)?;
        let cargo_runner = CargoRunner::new()?;
        cargo_runner.build_all(&Target::Other, false)?;
        git_runner.tag_release(&self.package, !self.real_publish)?;
        Ok(())
    }
}
