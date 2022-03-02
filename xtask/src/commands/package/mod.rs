#[cfg(target_os = "macos")]
mod macos;

use anyhow::{anyhow, bail, ensure, Context, Result};
use camino::Utf8PathBuf;
use std::path::Path;
use structopt::StructOpt;

use crate::commands;
use crate::packages::{BinaryCrate, PackageTag};
use crate::target::{Target, POSSIBLE_TARGETS};

const INCLUDE: &[&str] = &["README.md", "LICENSE"];

#[derive(Debug, StructOpt)]
pub struct Package {
    /// The target to build for
    #[structopt(long = "target", env = "XTASK_TARGET", default_value, possible_values = &POSSIBLE_TARGETS)]
    target: Target,

    /// Output tarball.
    #[structopt(long, default_value = "./artifacts")]
    output: Utf8PathBuf,

    /// Package tag to build. Currently only the `composition` tag produces binaries.
    #[structopt(long, env = "CIRCLE_TAG")]
    package: PackageTag,

    #[cfg(target_os = "macos")]
    #[structopt(flatten)]
    macos: macos::PackageMacos,

    /// The directory to put the stage repository
    #[structopt(long, default_value = "./stage", env = "XTASK_STAGE")]
    stage: Utf8PathBuf,
}

impl Package {
    pub fn run(&self, verbose: bool) -> Result<()> {
        if let Some(binary_crate) = self.package.package_group.get_binary() {
            let root_dir = commands::Dist {
                package: self.package.clone(),
                target: self.target.clone(),
                stage: self.stage.clone(),
            }
            .run(verbose)?;
            self.package.contains_correct_versions(&root_dir)?;
            self.create_tarball(&binary_crate, &root_dir)?;
            Ok(())
        } else {
            Err(anyhow!(
                "Could not find any binaries to package for package tag {}",
                &self.package
            ))
        }
    }

    fn create_tarball(&self, binary_crate: &BinaryCrate, root_dir: &Utf8PathBuf) -> Result<()> {
        let bin_name = binary_crate.to_string();
        let bin_name_with_suffix = format!("{}{}", bin_name, std::env::consts::EXE_SUFFIX);
        let release_path = root_dir
            .join("target")
            .join(self.target.to_string())
            .join("release")
            .join(&bin_name_with_suffix);

        ensure!(
            release_path.exists(),
            "Could not find binary at: {}",
            release_path
        );

        #[cfg(target_os = "macos")]
        self.macos
            .run(&release_path, &bin_name, &self.package.version)?;

        if !self.output.exists() {
            std::fs::create_dir_all(&self.output).context("Couldn't create output directory")?;
        }

        let output_path = if self.output.is_dir() {
            self.output.join(format!(
                "{}-v{}-{}.tar.gz",
                &bin_name, &self.package.version, self.target
            ))
        } else {
            bail!("--output must be a path to a directory, not a file.");
        };

        crate::info!("Creating tarball: {}", output_path);
        let mut file = flate2::write::GzEncoder::new(
            std::io::BufWriter::new(
                std::fs::File::create(&output_path).context("could not create TGZ file")?,
            ),
            flate2::Compression::default(),
        );
        let mut ar = tar::Builder::new(&mut file);
        crate::info!("Adding {} to tarball", release_path);
        ar.append_file(
            Path::new("dist").join(bin_name_with_suffix),
            &mut std::fs::File::open(release_path).context("could not open binary")?,
        )
        .context("could not add binary to TGZ archive")?;

        for filename in INCLUDE {
            let include_file = binary_crate.get_publish_src_path(root_dir)?.join(filename);
            crate::info!("Adding {} to tarball", &include_file);
            ar.append_file(
                Path::new("dist").join(&filename),
                &mut std::fs::File::open(include_file).context("could not open file")?,
            )
            .context("could not add file to TGZ archive")?;
        }

        ar.finish().context("could not finish TGZ archive")?;

        Ok(())
    }
}
