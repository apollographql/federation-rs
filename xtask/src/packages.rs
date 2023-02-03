use anyhow::{anyhow, Context, Error, Result};
use camino::Utf8PathBuf;
use semver::Version;

use crate::{target::POSSIBLE_TARGETS, utils::PKG_PROJECT_ROOT};

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

    fn contains_correct_versions(&self) -> Result<()> {
        let root_dir = self.get_workspace_dir()?;
        validate_cargo_toml(
            &root_dir,
            &self.version.to_string(),
            &self.package_group.get_library().to_string(),
        )?;
        if let Some(binary_crate) = self.package_group.get_binary() {
            validate_cargo_toml(
                &root_dir,
                &self.version.to_string(),
                &binary_crate.to_string(),
            )?;
        }
        Ok(())
    }

    pub(crate) fn get_workspace_dir(&self) -> Result<Utf8PathBuf> {
        match self.package_group {
            PackageGroup::Composition => match self.version.major {
                0 => Ok(PKG_PROJECT_ROOT.join("federation-1")),
                2 => Ok(PKG_PROJECT_ROOT.join("federation-2")),
                _ => Err(anyhow!("composition version must be 0 or 2")),
            },
            PackageGroup::RouterBridge => Ok(PKG_PROJECT_ROOT.join("federation-2")),
            PackageGroup::ApolloFederationTypes => Ok(PKG_PROJECT_ROOT.clone()),
        }
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
        err = err.with_context(|| format!("name {real_name} != {expected_name}"));
        is_err = true;
    }
    if real_version.to_string() != expected_version {
        err = err.with_context(|| format!("version {real_version} != {expected_version}"));
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
        let package_tag = if v.len() == 2 {
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
        }?;
        package_tag.contains_correct_versions()?;
        Ok(package_tag)
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
            "harmonizer" | "supergraph" => Err(anyhow!("{} is not a valid package group. you probably want the 'composition' prefix instead.", s)),
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

    fn get_required_artifact_files(&self, version: Version) -> Vec<String> {
        let mut required_artifacts = Vec::with_capacity(POSSIBLE_TARGETS.len());
        for target_triple in POSSIBLE_TARGETS {
            required_artifacts.push(format!("{}-v{}-{}.tar.gz", &self, &version, &target_triple))
        }
        required_artifacts.push("LICENSE".to_string());
        required_artifacts.push("sha1sums.txt".to_string());
        required_artifacts.push("sha256sums.txt".to_string());
        required_artifacts.push("md5sums.txt".to_string());
        required_artifacts
    }

    pub(crate) fn assert_includes_required_artifacts(
        &self,
        version: Version,
        artifacts_dir: &Utf8PathBuf,
    ) -> Result<()> {
        let required_artifact_files = match self {
            Self::Supergraph => self.get_required_artifact_files(version),
        };

        let mut existing_artifact_files = Vec::new();
        if let Ok(artifacts_contents) = fs::read_dir(artifacts_dir) {
            for artifact in artifacts_contents {
                let artifact = artifact?;
                let file_type = artifact.file_type()?;
                let name = artifact.file_name().to_string_lossy().to_string();
                if file_type.is_file() {
                    existing_artifact_files.push(name);
                } else if file_type.is_dir() {
                    return Err(anyhow!("Encountered unexpected dir {}. Please remove it before re-running this command.", &name));
                }
            }
        } else {
            return Err(anyhow!(
                "{} must exist. it must contain these files {:?}",
                artifacts_dir,
                &required_artifact_files
            ));
        }
        if existing_artifact_files.iter().all(|ef| {
            if required_artifact_files.contains(ef) {
                crate::info!("confirmed {} exists", ef);
                true
            } else {
                crate::info!(
                    "Found superfluous artifact file {} when publishing. Either add it to the list of required artifact files or ensure the artifact is not created.",
                    ef
                );
                false
            }
        }) {
            Ok(())
        } else {
            Err(anyhow!("Could not find all required artifact files."))
        }
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
