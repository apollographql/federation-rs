#[cfg(target_os = "macos")]
mod macos;

use anyhow::{bail, ensure, Context, Result};
use log::info;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

use crate::commands::Dist;
use crate::packages::{PackageGroup, PackageTag};
use crate::target::{Target, POSSIBLE_TARGETS};

const INCLUDE: &[&str] = &["README.md", "LICENSE"];

#[derive(Debug, StructOpt)]
pub struct Package {
    /// The target to build for
    #[structopt(long = "target", env = "XTASK_TARGET", default_value, possible_values = &POSSIBLE_TARGETS)]
    target: Target,

    /// Output tarball.
    #[structopt(long, default_value = "artifacts")]
    output: PathBuf,

    /// Package tag to build. Currently only the `composition` tag produces binaries.
    #[structopt(long, env = "CIRCLE_TAG")]
    package: PackageTag,

    #[cfg(target_os = "macos")]
    #[structopt(flatten)]
    macos: macos::PackageMacos,

    /// Builds without the --release flag
    #[structopt(long)]
    debug: bool,
}

impl Package {
    pub fn run(&self) -> Result<()> {
        Dist {
            target: self.target.clone(),
            debug: self.debug,
        }
        .run()
        .context("Could not build package")?;
        self.package_tarball()?;
        Ok(())
    }

    fn package_tarball(&self) -> Result<()> {
        let package = &self.package;
        if !matches!(package.package_group, PackageGroup::Composition) {
            bail!("Only the `composition` package group can be packaged");
        }
        let bin_name = "supergraph";
        let bin_name_with_suffix = format!("{bin_name}{}", std::env::consts::EXE_SUFFIX);
        let release_path = Path::new("target")
            .join(self.target.to_string())
            .join("release")
            .join(&bin_name_with_suffix);

        ensure!(
            release_path.exists(),
            "Could not find binary at: {}",
            release_path.display()
        );

        #[cfg(target_os = "macos")]
        self.macos
            .run(&release_path, bin_name, &self.package.version)?;

        if !self.output.exists() {
            std::fs::create_dir_all(&self.output).context("Couldn't create output directory")?;
        }

        let output_path = if self.output.is_dir() {
            self.output.join(format!(
                "{bin_name}-v{}-{}.tar.gz",
                &self.package.version, self.target
            ))
        } else {
            bail!("--output must be a path to a directory, not a file.");
        };

        info!("Creating tarball: {}", output_path.display());
        let mut file = flate2::write::GzEncoder::new(
            std::io::BufWriter::new(
                std::fs::File::create(&output_path).context("could not create TGZ file")?,
            ),
            flate2::Compression::default(),
        );
        let mut ar = tar::Builder::new(&mut file);
        info!("Adding {} to tarball", release_path.display());
        ar.append_file(
            Path::new("dist").join(bin_name_with_suffix),
            &mut std::fs::File::open(release_path).context("could not open binary")?,
        )
        .context("could not add binary to TGZ archive")?;

        for filename in INCLUDE {
            let include_file = Path::new("../../../../apollo-composition").join(filename);
            info!("Adding {} to tarball", &include_file.display());
            ar.append_file(
                Path::new("dist").join(filename),
                &mut std::fs::File::open(include_file).context("could not open file")?,
            )
            .context("could not add file to TGZ archive")?;
        }

        ar.finish().context("could not finish TGZ archive")?;
        Ok(())
    }
}
