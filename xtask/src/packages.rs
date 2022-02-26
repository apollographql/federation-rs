use anyhow::{anyhow, Context, Error, Result};
use camino::Utf8PathBuf;
use semver::Version;

use std::{fmt, fs, str::FromStr};

#[derive(Debug, Clone)]
pub(crate) struct PackageTag {
    pub(crate) package_group: PackageGroup,
    pub(crate) version: Version,
}

impl PackageTag {
    pub(crate) fn all_tags(&self) -> Vec<String> {
        self.package_group
            .get_tag_prefixes()
            .iter()
            .map(|prefix| format!("{}@v{}", prefix, &self.version))
            .collect()
    }

    pub(crate) fn contains_correct_versions(&self, root_dir: &Utf8PathBuf) -> Result<()> {
        validate_cargo_toml(
            root_dir,
            &self.version.to_string(),
            &self.package_group.get_library().to_string(),
        )?;
        if let Some(binary_crate) = self.package_group.get_binary() {
            validate_cargo_toml(
                root_dir,
                &self.version.to_string(),
                &binary_crate.to_string(),
            )?;
        }
        Ok(())
    }
}

fn validate_cargo_toml(
    root_dir: &Utf8PathBuf,
    expected_version: &str,
    expected_name: &str,
) -> Result<()> {
    let cargo_toml_path = root_dir.join(expected_name).join("Cargo.toml");
    let toml_contents = fs::read_to_string(&cargo_toml_path)?;

    let toml: toml::Value = toml_contents.parse().context("Cargo.toml is invalid")?;
    let real_version: Version = toml["package"]["version"]
        .as_str()
        .unwrap()
        .parse()
        .context("version in Cargo.toml is not valid semver")?;
    let real_name = toml["package"]["name"].as_str().unwrap();

    let mut err = Err(anyhow!(
        "There were a few problems with {}",
        &cargo_toml_path
    ));
    let mut is_err = false;
    if real_name != expected_name {
        err = err.with_context(|| format!("name {} != {}", real_name, expected_name));
        is_err = true;
    }
    if &real_version.to_string() != expected_version {
        err = err.with_context(|| format!("version {} != {}", real_version, expected_version));
        is_err = true
    }

    if is_err {
        err
    } else {
        Ok(())
    }
}

impl FromStr for PackageTag {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v: Vec<String> = s.trim().split("@v").map(|s| s.to_string()).collect();
        if v.len() == 2 {
            let package_group: PackageGroup = v[0].parse()?;
            let version: Version = v[1].parse()?;
            Ok(PackageTag {
                package_group,
                version,
            })
        } else {
            Err(anyhow!(
                "package tag must be in format '{{package_group}}@v{{package_version}}"
            ))
        }
    }
}

impl fmt::Display for PackageTag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}@v{}", self.package_group, self.version)
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum PackageGroup {
    Composition,
    ApolloFederationTypes,
    RouterBridge,
}

impl PackageGroup {
    pub(crate) fn get_crate_name(&self) -> String {
        match self {
            PackageGroup::Composition => "harmonizer",
            PackageGroup::ApolloFederationTypes => "apollo-federation-types",
            PackageGroup::RouterBridge => "router-bridge",
        }
        .to_string()
    }

    pub(crate) fn get_tag_prefixes(&self) -> Vec<String> {
        match self {
            PackageGroup::Composition => vec!["composition", "harmonizer", "supergraph"],
            PackageGroup::ApolloFederationTypes => vec!["apollo-federation-types"],
            PackageGroup::RouterBridge => vec!["router-bridge"],
        }
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    pub(crate) fn get_library(&self) -> LibraryCrate {
        match self {
            PackageGroup::Composition => LibraryCrate::Harmonizer,
            PackageGroup::ApolloFederationTypes => LibraryCrate::ApolloFederationTypes,
            PackageGroup::RouterBridge => LibraryCrate::RouterBridge,
        }
    }

    pub(crate) fn get_binary(&self) -> Option<BinaryCrate> {
        match self {
            PackageGroup::Composition => Some(BinaryCrate::Supergraph),
            _ => None,
        }
    }
}

impl FromStr for PackageGroup {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "composition" => Ok(PackageGroup::Composition),
            "apollo-federation-types" => Ok(PackageGroup::ApolloFederationTypes),
            "router-bridge" => Ok(PackageGroup::RouterBridge),
            "harmonizer" | "supergraph" => Err(anyhow!("{} is not a valid package group. you probably want to tag a 'composition' release instead.", s)),
            _ => Err(anyhow!("{} is not a valid package group", s)),
        }
    }
}

impl fmt::Display for PackageGroup {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PackageGroup::Composition => "composition",
                PackageGroup::ApolloFederationTypes => "apollo-federation-types",
                PackageGroup::RouterBridge => "router-bridge",
            }
        )
    }
}

pub(crate) enum LibraryCrate {
    Harmonizer,
    ApolloFederationTypes,
    RouterBridge,
}

impl fmt::Display for LibraryCrate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LibraryCrate::Harmonizer => "harmonizer",
                LibraryCrate::RouterBridge => "router-bridge",
                LibraryCrate::ApolloFederationTypes => "apollo-federation-types",
            }
        )
    }
}

pub(crate) enum BinaryCrate {
    Supergraph,
}

impl BinaryCrate {
    pub(crate) fn get_publish_src_path(&self, parent_dir: &Utf8PathBuf) -> Result<Utf8PathBuf> {
        let src = parent_dir.join(self.to_string());
        let _ = fs::read_dir(&src)?;
        Ok(src)
    }
}

impl fmt::Display for BinaryCrate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BinaryCrate::Supergraph => "supergraph",
            }
        )
    }
}
