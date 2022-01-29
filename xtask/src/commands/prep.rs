use crate::tools::CargoRunner;

use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;
use fs_extra::dir::CopyOptions;
use serde::Serialize;
use structopt::StructOpt;

use std::{
    env, fs,
    str::{self, FromStr},
};

#[derive(Debug, StructOpt)]
pub struct Prep {
    #[structopt(short = "z", long, possible_values = &["0", "2"])]
    harmonizer_version: HarmonizerVersion,
}

#[derive(Debug, Serialize)]
enum HarmonizerVersion {
    Zero,
    Two,
}

impl HarmonizerVersion {
    fn get_name(&self) -> &str {
        match self {
            Self::Zero => "harmonizer-0",
            Self::Two => "harmonizer-2",
        }
    }

    fn get_other_name(&self) -> String {
        match &self {
            Self::Two => Self::Zero,
            Self::Zero => Self::Two,
        }
        .get_name()
        .to_string()
    }
}

impl FromStr for HarmonizerVersion {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "0" => Ok(Self::Zero),
            "2" => Ok(Self::Two),
            _ => Err(anyhow!("Invalid harmonizer version.")),
        }
    }
}

impl Prep {
    pub fn run(&self, verbose: bool) -> Result<()> {
        let cargo_runner = CargoRunner::new(verbose)?;
        cargo_runner
            .build()
            .context("Could not build federation-rs")?;

        let current_dir: Utf8PathBuf = env::current_dir()?.try_into()?;
        let mut copy_options = CopyOptions::new();
        copy_options.overwrite = true;
        copy_options.copy_inside = true;

        let stage_dir = current_dir.join("stage");
        crate::info!("copying entire directory into ./stage");
        let _ = fs_extra::dir::remove(&stage_dir);
        fs_extra::dir::copy(&current_dir, &stage_dir, &copy_options)
            .context("Could not create ./stage from current directory")?;

        // rename the Cargo.publish.toml (auto-generated in build_harmonizer.rs) to Cargo.toml because it's go time.
        fs::rename(
            stage_dir.join("Cargo.publish.toml"),
            stage_dir.join("Cargo.toml"),
        )
        .context("Could not rename workspace Cargo.publish.toml to Cargo.toml")?;

        let this_harmonizer = self.harmonizer_version.get_name();
        let other_harmonizer = self.harmonizer_version.get_other_name();

        crate::info!("removing `{}` from the stage", other_harmonizer);

        // we won't be publishing the other version of harmonizer,
        // get it out of here!
        fs::remove_dir_all(stage_dir.join(&other_harmonizer))
            .with_context(|| format!("Could not remove {}", other_harmonizer))?;

        let harmonizer_src = stage_dir.join(this_harmonizer);
        let harmonizer_dest = stage_dir.join("harmonizer");

        crate::info!(
            "promoting `{}` to center stage as the one true `harmonizer`",
            this_harmonizer
        );

        // move the version of harmonizer we're publishing from harmonizer-x to harmonizer
        fs::rename(&harmonizer_src, &harmonizer_dest).with_context(|| {
            format!(
                "Could not rename `{}` to `{}`",
                harmonizer_src, harmonizer_dest
            )
        })?;

        // rename the Cargo.publish.toml (auto-generated in build_harmonizer.rs) to Cargo.toml because it's go time.
        fs::rename(
            harmonizer_dest.join("Cargo.publish.toml"),
            harmonizer_dest.join("Cargo.toml"),
        )
        .context("Could not rename Cargo.publish.toml to Cargo.toml")?;

        fs::remove_dir_all(stage_dir.join("target")).context("Could not remove ./stage/target")?;

        let cargo_runner = CargoRunner::new_with_path(verbose, stage_dir)?;
        cargo_runner.cargo_exec(vec!["publish", "--dry-run", "--allow-dirty"], vec![])?;

        Ok(())
    }
}
