use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use structopt::StructOpt;

use crate::tools::{CargoRunner, GitRunner};

#[derive(Debug, StructOpt)]
pub(crate) struct Publish {
    #[structopt(long, default_value = "./artifacts")]
    input: Utf8PathBuf,
}

impl Publish {
    pub fn run(&self, verbose: bool) -> Result<()> {
        let git_runner = GitRunner::new(true)?;

        // Check if (git) HEAD is pointing to a package tag
        //
        // NOTE: typically this will only succeed after running `cargo xtask tag --package {package} --real-publish`
        //       cargo xtask publish is executed by CircleCI
        let package_tag = git_runner
            .get_package_tag()
            .context("There are no valid package tags pointing to HEAD.")?;

        let workspace_directory = package_tag.get_workspace_dir().with_context(|| {
            format!(
                "Could not find the workspace directory for {}",
                &package_tag,
            )
        })?;

        if let Some(binary_crate) = package_tag.package_group.get_binary() {
            // before publishing, make sure we have all of the artifacts in place
            // this should have been done for us already by `cargo xtask package` running on all
            // of the different architectures, but let's make sure.
            binary_crate.assert_includes_required_artifacts(package_tag.version, &self.input)?;
        };

        // currently all packages have a library so just publish them.
        // if we ever wanted to publish just a binary without a library
        // to accompany it, change the function signature to
        // PackageGroup::get_library(&self) -> Option<LibraryCrate>
        // and handle it here.

        let cargo_runner = CargoRunner::new(verbose)?;
        cargo_runner.publish(
            &package_tag.package_group.get_library(),
            &workspace_directory,
        )?;

        Ok(())
    }
}
