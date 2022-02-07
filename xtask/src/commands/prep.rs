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

    #[structopt(skip)]
    env: StageEnv,
}

#[derive(Debug, Serialize)]
enum HarmonizerVersion {
    Zero,
    Two,
}

#[derive(Debug, Serialize)]
struct StageEnv {
    current_dir: Utf8PathBuf,
    stage_dir: Utf8PathBuf,
    pub_harmonizer_dir: Utf8PathBuf,
    pub_supergraph_dir: Utf8PathBuf,
}

impl Default for StageEnv {
    fn default() -> Self {
        let current_dir =
            Utf8PathBuf::try_from(env::current_dir().expect("Could not find current directory."))
                .expect("Current directory is not valid UTF-8.");
        let stage_dir = current_dir.join("stage");
        let pub_harmonizer_dir = stage_dir.join("harmonizer");
        let pub_supergraph_dir = stage_dir.join("supergraph");

        Self {
            current_dir,
            stage_dir,
            pub_harmonizer_dir,
            pub_supergraph_dir,
        }
    }
}

impl HarmonizerVersion {
    fn get_name(&self) -> String {
        match self {
            Self::Zero => "harmonizer-0".to_string(),
            Self::Two => "harmonizer-2".to_string(),
        }
    }

    fn get_other_name(&self) -> String {
        match &self {
            Self::Two => Self::Zero,
            Self::Zero => Self::Two,
        }
        .get_name()
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

        self.set_stage()
            .with_context(|| format!("Could not create `{}`", &self.env.stage_dir))?;

        let cargo_runner = CargoRunner::new_with_path(verbose, &self.env.stage_dir)?;
        cargo_runner
            .cargo_exec(
                vec![
                    "publish",
                    "-p",
                    "apollo-federation-types",
                    "--dry-run",
                    "--allow-dirty",
                ],
                vec![],
            )
            .context("Cannot currently publish apollo-federation-types.")?;
        cargo_runner.cargo_exec(
            vec!["publish", "-p", "harmonizer", "--dry-run", "--allow-dirty"],
            vec![],
        ).context("Cannot currently publish harmonizer. This is likely because the version of apollo-federation-types was bumped and it needs released.")?;

        Ok(())
    }

    fn set_stage(&self) -> Result<()> {
        crate::info!("setting the stage for publishing");

        self.init_stage()
            .context("Could not copy current directory to stage")?;

        prepare_publish_manifest(&self.env.stage_dir)
            .context("Could not prepare workspace publish manfiest")?;

        self.only_use_one_harmonizer()
            .context("Could not promote the correct harmonizer version")?;

        self.only_use_one_supergraph()
            .context("Could not promote the correct supergraph version")?;

        Ok(())
    }

    fn only_use_one_harmonizer(&self) -> Result<()> {
        let this_harmonizer = self.harmonizer_version.get_name();
        let other_harmonizer = self.harmonizer_version.get_other_name();
        self.remove_version(&other_harmonizer)?;
        self.promote_harmonizer_version(&this_harmonizer)?;
        Ok(())
    }

    fn only_use_one_supergraph(&self) -> Result<()> {
        let this_supergraph = self
            .harmonizer_version
            .get_name()
            .replace("harmonizer", "supergraph");
        let other_supergraph = self
            .harmonizer_version
            .get_other_name()
            .replace("harmonizer", "supergraph");
        self.remove_version(&other_supergraph)?;
        self.promote_supergraph_version(&this_supergraph)?;
        Ok(())
    }

    fn init_stage(&self) -> Result<()> {
        crate::info!(
            "copying `{}` into `{}`",
            &self.env.current_dir,
            &self.env.stage_dir
        );

        let mut copy_options = CopyOptions::new();
        copy_options.overwrite = true;
        copy_options.copy_inside = true;

        let _ = fs_extra::dir::remove(&self.env.stage_dir);
        fs_extra::dir::copy(&self.env.current_dir, &self.env.stage_dir, &copy_options)
            .with_context(|| {
                format!(
                    "Could not copy `{}` to `{}`",
                    &self.env.current_dir, &self.env.stage_dir
                )
            })?;
        Ok(())
    }

    fn promote_harmonizer_version(&self, dev_harmonizer_dir: &str) -> Result<()> {
        let harmonizer_src = &self.env.stage_dir.join(dev_harmonizer_dir);
        let harmonizer_dest = &self.env.pub_harmonizer_dir;

        crate::info!("renaming `{}` to `{}`", harmonizer_src, harmonizer_dest);

        // move the version of harmonizer we're publishing from harmonizer-x to harmonizer
        fs::rename(harmonizer_src, harmonizer_dest).with_context(|| {
            format!(
                "Could not rename `{}` to `{}`",
                harmonizer_src, harmonizer_dest
            )
        })?;

        prepare_publish_manifest(harmonizer_dest)?;
        Ok(())
    }

    fn remove_version(&self, dev_harmonizer_dir: &str) -> Result<()> {
        let remove_dir = &self.env.stage_dir.join(&dev_harmonizer_dir);
        crate::info!("deleting `{}`", remove_dir);

        // we won't be publishing the other version of harmonizer,
        // get it out of here!
        fs::remove_dir_all(remove_dir)
            .with_context(|| format!("Could not remove `{}`", remove_dir))?;
        Ok(())
    }

    fn promote_supergraph_version(&self, dev_supergraph_dir: &str) -> Result<()> {
        let supergraph_src = &self.env.stage_dir.join(dev_supergraph_dir);
        let supergraph_dest = &self.env.pub_supergraph_dir;

        crate::info!("renaming `{}` to `{}`", supergraph_src, supergraph_dest);

        // move the version of harmonizer we're publishing from harmonizer-x to harmonizer
        fs::rename(supergraph_src, supergraph_dest).with_context(|| {
            format!(
                "Could not rename `{}` to `{}`",
                supergraph_src, supergraph_dest
            )
        })?;

        prepare_publish_manifest(supergraph_dest)?;
        Ok(())
    }
}

// replace the Cargo.toml in a given directory with the Cargo.publish.toml
fn prepare_publish_manifest(dir: &Utf8PathBuf) -> Result<()> {
    if !dir.is_dir() {
        Err(anyhow!("`{}` is not a directory", dir))
    } else {
        let src = dir.join("Cargo.publish.toml");
        let dest = dir.join("Cargo.toml");

        if src.is_file() && dest.is_file() {
            crate::info!("renaming `{}` to `{}`", &src, &dest);
            fs::rename(&src, &dest)
                .with_context(|| format!("Could not rename `{}` to `{}`", &src, &dest))?;
            Ok(())
        } else {
            Err(anyhow!("`{}` must contain a `Cargo.toml` AND a `Cargo.publish.toml`. There might be something wrong with build_harmonizer.rs?", dir))
        }
    }
}
