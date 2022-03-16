use anyhow::{anyhow, Result};
use camino::Utf8PathBuf;
use structopt::StructOpt;

use crate::packages::PackageGroup;
use crate::tools::{CargoRunner, GitRunner};
use crate::utils::PKG_PROJECT_ROOT;

use std::fs;

#[derive(Debug, StructOpt)]
pub(crate) struct Publish {
    #[structopt(long, default_value = "./artifacts")]
    input: Utf8PathBuf,
}

impl Publish {
    pub fn run(&self, verbose: bool) -> Result<()> {
        let git_runner = GitRunner::new(true)?;
        let package_tag = git_runner.get_package_tag()?;

        package_tag.contains_correct_versions()?;

        match package_tag.package_group {
            PackageGroup::Composition => {
                // before publishing, make sure we have all of the artifacts in place
                // this should have been done for us already by `cargo xtask package` running on all
                // of the different architectures, but let's make sure.
                let root_dir = package_tag.get_workspace_dir()?;
                let required_artifact_files = vec![
                    format!(
                        "supergraph-v{}-x86_64-unknown-linux-gnu.tar.gz",
                        &package_tag.version
                    ),
                    format!(
                        "supergraph-v{}-x86_64-apple-darwin.tar.gz",
                        &package_tag.version
                    ),
                    format!(
                        "supergraph-v{}-x86_64-pc-windows-msvc.tar.gz",
                        &package_tag.version
                    ),
                    "sha1sums.txt".to_string(),
                    "sha256sums.txt".to_string(),
                    "md5sums.txt".to_string(),
                ];
                let mut existing_artifact_files = Vec::new();
                if let Ok(artifacts_contents) = fs::read_dir(&self.input) {
                    for artifact in artifacts_contents {
                        let artifact = artifact?;
                        let file_type = artifact.file_type()?;
                        if file_type.is_file() {
                            existing_artifact_files
                                .push(artifact.file_name().to_string_lossy().to_string());
                        }
                    }
                } else {
                    return Err(anyhow!(
                        "{} must exist. it must contain these files {:?}",
                        &self.input,
                        &required_artifact_files
                    ));
                }
                assert!(existing_artifact_files.iter().all(|ef| {
                    if required_artifact_files.contains(ef) {
                        crate::info!("confirmed {} exists", ef);
                        true
                    } else {
                        crate::info!(
                            "we require {} before publishing, but it does not exist.",
                            ef
                        );
                        false
                    }
                }));

                let cargo_runner = CargoRunner::new(verbose)?;
                cargo_runner.publish(&package_tag.package_group.get_library(), &root_dir)?;
                Ok(())
            }
            PackageGroup::ApolloFederationTypes | PackageGroup::RouterBridge => {
                let cargo_runner = CargoRunner::new(verbose)?;
                cargo_runner
                    .publish(&package_tag.package_group.get_library(), &PKG_PROJECT_ROOT)?;
                Ok(())
            }
        }
    }
}
