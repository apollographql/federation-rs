use std::process::{ExitStatus, Output};
use std::str::FromStr;

use crate::{packages::PackageTag, tools::Runner};

use anyhow::{anyhow, Context, Result};
use log::info;

pub(crate) struct GitRunner {
    runner: Runner,
}

impl GitRunner {
    pub(crate) fn new() -> Result<Self> {
        let runner = Runner::new("git");

        Ok(GitRunner { runner })
    }

    pub(crate) fn can_tag(&self) -> Result<()> {
        self.exec(&["fetch"])?;
        // let branch_name =
        //     String::from_utf8_lossy(&self.exec_with_output(&["branch", "--show-current"])?.stdout)
        //         .trim()
        //         .to_string();
        let status_msg =
            String::from_utf8_lossy(&self.exec_with_output(&["status", "-uno"])?.stdout)
                .trim()
                .to_string();
        // if branch_name != "main" {
        //     Err(anyhow!(
        //         "You must run this command from the latest commit of the `main` branch, it looks like you're on {}", &branch_name
        //     ))
        // } else
        if status_msg.contains("Changes not staged for commit") {
            Err(anyhow!(
                "Your working tree is dirty, please fix this before releasing."
            ))
        } else if status_msg.contains("out of date") {
            Err(anyhow!("Your local `main` is out of date with the remote"))
        } else {
            Ok(())
        }
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

    // takes a PackageTag and kicks off a release in CircleCI
    pub(crate) fn tag_release(&self, package_tag: &PackageTag, dry_run: bool) -> Result<()> {
        if !dry_run {
            // create all the git tags we need from the PackageTag, and push up
            // only the tags we created here
            for tag in package_tag.all_tags() {
                self.exec(&["tag", "-a", &tag, "-m", &tag]).context("If you want to re-publish this version, first delete the tag in GitHub at https://github.com/apollographql/federation-rs/current_git_tags")?;
                // Fully qualify the tag name to avoid ambiguity with branches
                let refs_tags_tag = format!("refs/tags/{}", &tag);
                self.exec(&["push", "origin", refs_tags_tag.as_str(), "--no-verify"])?;
            }
            info!("kicked off release build: 'https://app.circleci.com/pipelines/github/apollographql/federation-rs'");
        } else {
            // show what we would do with the tags, this is helpful for debugging
            info!("would run `git tag -d $(git tag) && git fetch --tags");
            for tag in package_tag.all_tags() {
                info!("would run `git tag -a {} -m {}", &tag, &tag);
            }
            info!("would run `git push --tags --no-verify`, which would kick off a release build at 'https://app.circleci.com/pipelines/github/apollographql/federation-rs'");
        }
        Ok(())
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
