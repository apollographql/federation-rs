use crate::{
    packages::PackageGroup,
    packages::PackageTag,
    target::{Target, POSSIBLE_TARGETS},
    tools::CargoRunner,
    utils::PKG_PROJECT_ROOT,
};

use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;
use fs_extra::dir::CopyOptions;
use serde::Serialize;
use structopt::StructOpt;

use std::{
    fs,
    str::{self, FromStr},
};

#[derive(Debug, StructOpt)]
pub(crate) struct Prep {
    #[structopt(long, env = "CIRCLE_TAG")]
    pub(crate) package: PackageTag,

    /// The target to build for
    #[structopt(long = "target", env = "XTASK_TARGET", default_value, possible_values = &POSSIBLE_TARGETS)]
    pub(crate) target: Target,

    /// The directory to put the stage repository
    #[structopt(long, default_value = "./stage", env = "XTASK_STAGE")]
    pub(crate) stage: Utf8PathBuf,
}

impl Prep {
    pub fn run(&self, verbose: bool) -> Result<Option<StageEnv>> {
        if let PackageGroup::Composition = self.package.package_group {
            let env = StageEnv::new(self.stage.clone());
            let harmonizer_version: HarmonizerVersion =
                self.package.version.major.to_string().parse()?;
            let mut cargo_runner = CargoRunner::new(verbose)?;
            cargo_runner
                .build(&self.target, true)
                .context("Could not build federation-rs")?;

            self.set_stage(&harmonizer_version, &env)
                .with_context(|| format!("Could not create `{}`", &env.stage_dir))?;
            Ok(Some(env))
        } else {
            Ok(None)
        }
    }

    fn set_stage(&self, harmonizer_version: &HarmonizerVersion, env: &StageEnv) -> Result<()> {
        crate::info!("setting the stage for publishing");

        self.init_stage(env)
            .context("Could not copy current directory to stage")?;

        prepare_publish_manifest(&env.stage_dir)
            .context("Could not prepare workspace publish manfiest")?;

        self.only_use_one_harmonizer(env, harmonizer_version)
            .context("Could not promote the correct harmonizer version")?;

        self.only_use_one_supergraph(env, harmonizer_version)
            .context("Could not promote the correct supergraph version")?;

        self.package.contains_correct_versions(&env.stage_dir)?;

        Ok(())
    }

    fn only_use_one_harmonizer(
        &self,
        env: &StageEnv,
        harmonizer_version: &HarmonizerVersion,
    ) -> Result<()> {
        let this_harmonizer = harmonizer_version.get_name();
        let other_harmonizer = harmonizer_version.get_other_name();
        self.remove_version(env, &other_harmonizer)?;
        self.promote_harmonizer_version(env, &this_harmonizer)?;
        Ok(())
    }

    fn only_use_one_supergraph(
        &self,
        env: &StageEnv,
        harmonizer_version: &HarmonizerVersion,
    ) -> Result<()> {
        let this_supergraph = harmonizer_version
            .get_name()
            .replace("harmonizer", "supergraph");
        let other_supergraph = harmonizer_version
            .get_other_name()
            .replace("harmonizer", "supergraph");
        self.remove_version(env, &other_supergraph)?;
        self.promote_supergraph_version(env, &this_supergraph)?;
        Ok(())
    }

    fn init_stage(&self, env: &StageEnv) -> Result<()> {
        crate::info!("copying `{}` into `{}`", *PKG_PROJECT_ROOT, env.stage_dir);

        let mut copy_options = CopyOptions::new();
        copy_options.overwrite = true;
        copy_options.copy_inside = true;

        let _ = fs_extra::dir::remove(&env.stage_dir);
        fs_extra::dir::copy(&*PKG_PROJECT_ROOT, &env.stage_dir, &copy_options).with_context(
            || {
                format!(
                    "Could not copy `{}` to `{}`",
                    *PKG_PROJECT_ROOT, env.stage_dir
                )
            },
        )?;
        Ok(())
    }

    fn promote_harmonizer_version(&self, env: &StageEnv, dev_harmonizer_dir: &str) -> Result<()> {
        let harmonizer_src = &env.stage_dir.join(dev_harmonizer_dir);
        let harmonizer_dest = &env.pub_harmonizer_dir;

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

    fn remove_version(&self, env: &StageEnv, dev_harmonizer_dir: &str) -> Result<()> {
        let remove_dir = &env.stage_dir.join(&dev_harmonizer_dir);
        crate::info!("deleting `{}`", remove_dir);

        // we won't be publishing the other version of harmonizer,
        // get it out of here!
        fs::remove_dir_all(remove_dir)
            .with_context(|| format!("Could not remove `{}`", remove_dir))?;
        Ok(())
    }

    fn promote_supergraph_version(&self, env: &StageEnv, dev_supergraph_dir: &str) -> Result<()> {
        let supergraph_src = &env.stage_dir.join(dev_supergraph_dir);
        let supergraph_dest = &env.pub_supergraph_dir;

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

// stuff below here is bad since it came before "package tags".
// it should probably be refactored but it works fine.

#[derive(Debug, Serialize)]
pub(crate) enum HarmonizerVersion {
    Zero,
    Two,
}

#[derive(Debug, Serialize)]
pub(crate) struct StageEnv {
    pub(crate) stage_dir: Utf8PathBuf,
    pub(crate) pub_harmonizer_dir: Utf8PathBuf,
    pub(crate) pub_supergraph_dir: Utf8PathBuf,
}

impl StageEnv {
    fn new(stage_dir: Utf8PathBuf) -> Self {
        let pub_harmonizer_dir = stage_dir.join("harmonizer");
        let pub_supergraph_dir = stage_dir.join("supergraph");

        Self {
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
