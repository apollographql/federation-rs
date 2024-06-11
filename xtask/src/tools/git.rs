use std::process::{ExitStatus, Output};
use std::str::FromStr;

use crate::{packages::PackageTag, tools::Runner};

use anyhow::{anyhow, Result};

pub(crate) struct GitRunner {
    runner: Runner,
}

impl GitRunner {
    pub(crate) fn new() -> Result<Self> {
        let runner = Runner::new("git");

        Ok(GitRunner { runner })
    }

    // this will update the tags we know about,
    // overwriting any local tags we may have
    // (such as an outdated `composition-latest-{0,2}`)
    fn fetch_remote_tags(&self) -> Result<()> {
        self.exec(&["fetch", "--tags", "--force"])?;
        Ok(())
    }

    // gets the current tags that point to HEAD
    pub(crate) fn get_head_tags(&self) -> Result<Vec<String>> {
        self.fetch_remote_tags()?;
        let output = self.exec_with_output(&["tag", "--points-at", "HEAD"])?;
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect())
    }

    // returns the `PackageTag` associated with a git tag that is currently pointing to HEAD
    // this is used by `cargo xtask publish` in CI to know what crate/binary to publish
    pub(crate) fn get_package_tag(&self) -> Result<PackageTag> {
        let current_git_tags = self.get_head_tags()?;
        for tag in &current_git_tags {
            // check if one of the current current_git_tags is a real package tag
            if let Ok(package_tag) = PackageTag::from_str(tag) {
                let desired_package_tags = package_tag.all_tags();

                // make sure we have all of the current_git_tags we need before proceeding
                if desired_package_tags
                    .iter()
                    .all(|desired_tag| current_git_tags.contains(desired_tag))
                {
                    return Ok(package_tag);
                }
            }
        }

        if current_git_tags.is_empty() {
            Err(anyhow!(
                "It doesn't look like there are any current_git_tags pointing to HEAD."
            ))
        } else {
            Err(anyhow!(
                "The tag(s) pointing to HEAD are invalid. current current_git_tags: {:?}",
                current_git_tags
            ))
        }
    }

    fn exec(&self, arguments: &[&str]) -> Result<ExitStatus> {
        self.runner
            .exec(arguments, &[], None, false)
            .map(|output| output.status)
    }

    fn exec_with_output(&self, arguments: &[&str]) -> Result<Output> {
        self.runner.exec(arguments, &[], None, true)
    }
}
