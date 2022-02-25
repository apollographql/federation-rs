use crate::{packages::PackageGroup, packages::PackageTag, tools::CargoRunner};

use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;
use fs_extra::dir::CopyOptions;
use semver::Version;
use serde::Serialize;
use structopt::StructOpt;

use std::{
    env, fs,
    str::{self, FromStr},
};

#[derive(Debug, StructOpt)]
pub(crate) struct Prep {
    #[structopt(long = "package")]
    pub(crate) package_tag: PackageTag,
}

#[derive(Debug, Serialize)]
pub(crate) enum HarmonizerVersion {
    Zero,
    Two,
}

#[derive(Debug, Serialize)]
pub(crate) struct StageEnv {
    pub(crate) current_dir: Utf8PathBuf,
    pub(crate) stage_dir: Utf8PathBuf,
    pub(crate) pub_harmonizer_dir: Utf8PathBuf,
    pub(crate) pub_supergraph_dir: Utf8PathBuf,
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
    pub fn run(&self, verbose: bool) -> Result<Option<StageEnv>> {
        if let PackageGroup::Composition = self.package_tag.package_group {
            let env = StageEnv::default();
            let harmonizer_version: HarmonizerVersion =
                self.package_tag.version.major.to_string().parse()?;
            let cargo_runner = CargoRunner::new(verbose)?;
            cargo_runner
                .build()
                .context("Could not build federation-rs")?;

            self.set_stage(&harmonizer_version, &env, &self.package_tag.version)
                .with_context(|| format!("Could not create `{}`", &env.stage_dir))?;
            Ok(Some(env))
        } else {
            Ok(None)
        }
    }

    fn set_stage(
        &self,
        harmonizer_version: &HarmonizerVersion,
        env: &StageEnv,
        expected_version: &Version,
    ) -> Result<()> {
        crate::info!("setting the stage for publishing");

        self.init_stage(env)
            .context("Could not copy current directory to stage")?;

        prepare_publish_manifest(&env.stage_dir)
            .context("Could not prepare workspace publish manfiest")?;

        self.only_use_one_harmonizer(env, harmonizer_version)
            .context("Could not promote the correct harmonizer version")?;

        self.only_use_one_supergraph(env, harmonizer_version)
            .context("Could not promote the correct supergraph version")?;

        self.validate_stage(env, expected_version).map_err(|e| {
            let _ = fs::remove_dir(&env.stage_dir);
            e
        })?;

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
        crate::info!("copying `{}` into `{}`", env.current_dir, env.stage_dir);

        let mut copy_options = CopyOptions::new();
        copy_options.overwrite = true;
        copy_options.copy_inside = true;

        let _ = fs_extra::dir::remove(&env.stage_dir);
        fs_extra::dir::copy(&env.current_dir, &env.stage_dir, &copy_options).with_context(
            || {
                format!(
                    "Could not copy `{}` to `{}`",
                    env.current_dir, env.stage_dir
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

        prepare_publish_manifest(&harmonizer_dest)?;
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

        prepare_publish_manifest(&supergraph_dest)?;
        Ok(())
    }

    fn validate_stage(&self, env: &StageEnv, expected_version: &Version) -> Result<()> {
        let harmonizer_toml_contents =
            fs::read_to_string(env.pub_harmonizer_dir.join("Cargo.toml"))
                .context("couldn't read harmonizer Cargo.toml")?;
        let supergraph_toml_contents =
            fs::read_to_string(env.pub_supergraph_dir.join("Cargo.toml"))
                .context("couldn't read supergraph Cargo.toml")?;

        let harmonizer_toml: toml::Value = harmonizer_toml_contents
            .parse()
            .context("harmonizer Cargo.toml is invalid")?;
        let supergraph_toml: toml::Value = supergraph_toml_contents
            .parse()
            .context("supergraph Cargo.toml is invalid")?;
        let harmonizer_real_version: Version = harmonizer_toml["package"]["version"]
            .as_str()
            .unwrap()
            .parse()
            .context("version in harmonizer Cargo.toml is not valid semver")?;
        let supergraph_real_version: Version = supergraph_toml["package"]["version"]
            .as_str()
            .unwrap()
            .parse()
            .context("version in supergraph Cargo.toml is not valid semver")?;
        let harmonizer_real_name = harmonizer_toml["package"]["name"].as_str().unwrap();
        let supergraph_real_name = supergraph_toml["package"]["name"].as_str().unwrap();
        if harmonizer_real_name != "harmonizer" {
            Err(anyhow!(
                "expected crate name 'harmonizer' but found crate name '{}'",
                harmonizer_real_name
            ))
        } else if supergraph_real_name != "supergraph" {
            Err(anyhow!(
                "expected crate name 'supergraph' but found crate name '{}'",
                supergraph_real_name
            ))
        } else if &harmonizer_real_version != expected_version {
            Err(anyhow!(
                "you must bump the harmonizer crate version before you can publish. Cargo.toml says {}, you passed {}",
                harmonizer_real_version,
                expected_version
            ))
        } else if supergraph_real_version != harmonizer_real_version {
            Err(anyhow!("supergraph version is not the same as the harmonizer crate version, you probably need to rebuild the project and rerun the prep command"))
        } else {
            Ok(())
        }
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
