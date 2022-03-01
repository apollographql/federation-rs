use anyhow::{anyhow, Context, Result};
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

    /// The directory to publish from. In CI this is ./artifacts/stage
    #[structopt(long, env = "XTASK_STAGE")]
    stage: Utf8PathBuf,
}

impl Publish {
    pub fn run(&self, verbose: bool) -> Result<()> {
        let git_runner = GitRunner::new(true)?;
        let package_tag = git_runner.get_package_tag()?;

        match package_tag.package_group {
            PackageGroup::Composition => {
                // before publishing, make sure we have all of the artifacts in place
                // this should have been done for us already by `cargo xtask package` running on all
                // of the different architectures, but let's make sure.
                let _ = fs::read_dir(&self.stage).context("{} does not exist")?;
                package_tag.contains_correct_versions(&self.stage)?;
                let mut required_artifact_subdirectories = vec![
                    format!(
                        "supergraph-v{}-x86_64-unknown-linux-gnu.tar.gz",
                        &package_tag.version
                    ),
                    format!(
                        "supergraph-v{}-x86_64-apple-darwin.tar.gz",
                        &package_tag.version
                    ),
                    format!(
                        "supergraph-v{}-pc-windows-msvc.tar.gz",
                        &package_tag.version
                    ),
                ];
                let mut required_artifact_files =
                    vec!["sha1sums.txt".to_string(), "sha256sums.txt".to_string()];
                let mut existing_artifact_subdirectories = Vec::new();
                let mut existing_artifact_files = Vec::new();
                if let Ok(artifacts_contents) = fs::read_dir(&self.input) {
                    for artifact in artifacts_contents {
                        let artifact = artifact?;
                        let file_type = artifact.file_type()?;
                        if file_type.is_dir() {
                            existing_artifact_subdirectories
                                .push(artifact.file_name().to_string_lossy().to_string());
                        } else if file_type.is_file() {
                            existing_artifact_files
                                .push(artifact.file_name().to_string_lossy().to_string());
                        }
                    }
                } else {
                    return Err(anyhow!("{} must exist. it must contain these subdirectories {:?} and these files {:?}", &self.input, &required_artifact_subdirectories, &required_artifact_files));
                }
                // sort to check for equality
                existing_artifact_files.sort();
                required_artifact_files.sort();
                existing_artifact_subdirectories.sort();
                required_artifact_subdirectories.sort();
                assert_eq!(
                    existing_artifact_subdirectories,
                    required_artifact_subdirectories
                );
                assert_eq!(existing_artifact_files, required_artifact_files);

                let cargo_runner = CargoRunner::new_with_path(verbose, &self.stage)?;
                cargo_runner.publish(&package_tag.package_group.get_crate_name())?;

                Ok(())
            }
            PackageGroup::ApolloFederationTypes | PackageGroup::RouterBridge => {
                package_tag.contains_correct_versions(&PKG_PROJECT_ROOT)?;
                let cargo_runner = CargoRunner::new(verbose)?;
                cargo_runner.publish(&package_tag.package_group.get_crate_name())?;
                Ok(())
            }
        }
    }
}
