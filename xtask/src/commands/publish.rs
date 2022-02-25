use anyhow::{anyhow, Result};
use easy_parallel::Parallel;
use structopt::StructOpt;

use crate::commands;
use crate::packages::PackageGroup;
use crate::tools::{CargoRunner, GitRunner};

#[derive(Debug, StructOpt)]
pub(crate) struct Publish {
    #[structopt(long)]
    skip_tests: bool,

    #[structopt(long)]
    skip_lints: bool,
}

impl Publish {
    pub fn run(&self, verbose: bool) -> Result<()> {
        let git_runner = GitRunner::new(true)?;
        let package_tag = git_runner.get_package_tag()?;
        let mut stage_env = None;

        let (prep_results, ()) = Parallel::new()
            .add(|| {
                if !self.skip_tests {
                    commands::Test {}.run(verbose)
                } else {
                    Ok(())
                }
            })
            .add(|| {
                if !self.skip_lints {
                    commands::Lint {}.run(verbose)
                } else {
                    Ok(())
                }
            })
            .add(|| {
                stage_env = commands::Prep {
                    package_tag: package_tag.clone(),
                }
                .run(verbose)?;
                Ok(())
            })
            .finish(|| crate::info!("Running tests, lints, and prep in background"));

        for prep_result in prep_results {
            prep_result?
        }

        match package_tag.package_group {
            PackageGroup::Composition => {
                if let Some(stage_env) = stage_env {
                    let cargo_runner = CargoRunner::new_with_path(verbose, stage_env.stage_dir)?;
                    cargo_runner.publish(&package_tag)?;
                    // TODO: handle "artifacts" creation here for the supergraph binary
                    Ok(())
                } else {
                    Err(anyhow!("`cargo xtask prep` did not create a stage directory for the composition package group"))
                }
            }
            PackageGroup::ApolloFederationTypes | PackageGroup::RouterBridge => {
                let cargo_runner = CargoRunner::new(verbose)?;
                cargo_runner.publish(&package_tag)?;
                Ok(())
            }
        }?;
        Ok(())
    }
}
