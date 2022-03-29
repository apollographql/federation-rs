use std::{convert::TryFrom, env, str::FromStr};

use crate::{packages::PackageTag, tools::Runner, utils::CommandOutput};

use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;

pub(crate) struct GitRunner {
    repo_path: Utf8PathBuf,
    runner: Runner,
}

impl GitRunner {
    pub(crate) fn new(verbose: bool) -> Result<Self> {
        let runner = Runner::new("git", verbose)?;
        let repo_path = Utf8PathBuf::try_from(env::current_dir()?)?;

        Ok(GitRunner { runner, repo_path })
    }

    pub(crate) fn can_tag(&self) -> Result<()> {
        self.exec(&["fetch"])?;
        let branch_name = self
            .exec(&["branch", "--show-current"])?
            .stdout
            .trim()
            .to_string();
        let status_msg = self.exec(&["status", "-uno"])?.stdout.trim().to_string();
        if branch_name != "main" {
            Err(anyhow!(
                "You must run this command from the latest commit of the `main` branch, it looks like you're on {}", &branch_name
            ))
        } else if status_msg.contains("Changes not staged for commit") {
            Err(anyhow!(
                "Your working tree is dirty, please fix this before releasing."
            ))
        } else if status_msg.contains("out of date") {
            Err(anyhow!("Your local `main` is out of date with the remote"))
        } else {
            Ok(())
        }
    }

    pub(crate) fn get_tags(&self) -> Result<Vec<String>> {
        self.exec(&["fetch", "--tags"])?;
        Ok(self
            .exec(&["tag"])?
            .stdout
            .lines()
            .map(|s| s.to_string())
            .collect())
    }

    pub(crate) fn get_head_tags(&self) -> Result<Vec<String>> {
        self.exec(&["fetch", "--tags"])?;
        Ok(self
            .exec(&["tag", "--points-at", "HEAD"])?
            .stdout
            .lines()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }

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

    pub(crate) fn tag_release(&self, package_tag: &PackageTag, dry_run: bool) -> Result<()> {
        self.can_tag()?;
        self.exec(&["pull"])?;
        if !dry_run {
            for local_tag in self.get_tags()? {
                self.exec(&["tag", "-d", &local_tag])?;
            }
            self.exec(&["fetch", "--tags"])?;
            for tag in package_tag.all_tags() {
                self.exec(&["tag", "-a", &tag, "-m", &tag]).context("If you want to re-publish this version, first delete the tag in GitHub at https://github.com/apollographql/federation-rs/current_git_tags")?;
            }
            self.exec(&["push", "--tags", "--no-verify"])?;
        } else {
            crate::info!("would run `git tag -d $(git tag) && git fetch --tags");
            for tag in package_tag.all_tags() {
                crate::info!("would run `git tag -a {} -m {}", &tag, &tag);
            }
            crate::info!("would run `git push --tags --no-verify`");
        }
        Ok(())
    }

    fn exec(&self, arguments: &[&str]) -> Result<CommandOutput> {
        self.runner.exec(arguments, &self.repo_path, None)
    }
}
