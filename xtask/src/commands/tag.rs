use anyhow::Result;
use structopt::StructOpt;

use crate::{packages::PackageTag, tools::GitRunner};

#[derive(Debug, StructOpt)]
pub(crate) struct Tag {
    /// this command does a dry run tag by default,
    /// to really run it, pass --real-publish
    #[structopt(long)]
    pub(crate) real_publish: bool,

    #[structopt(long = "package")]
    pub(crate) package_tag: PackageTag,
}

impl Tag {
    pub(crate) fn run(&self, verbose: bool) -> Result<()> {
        let git_runner = GitRunner::new(verbose)?;
        git_runner.tag_release(&self.package_tag, !self.real_publish)
    }
}
