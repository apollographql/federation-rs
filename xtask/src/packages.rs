use anyhow::{anyhow, Context, Error, Result};
use semver::Version;

use crate::target::POSSIBLE_TARGETS;

use log::info;
use std::path::Path;
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
        validate_cargo_toml(
            &self.version.to_string(),
            &self.package_group.get_library().to_string(),
        )?;
        if matches!(self.package_group, PackageGroup::Composition) {
            validate_cargo_toml(&self.version.to_string(), "composition")?;
        }
        Ok(())
    }
}

fn validate_cargo_toml(expected_version: &str, expected_name: &str) -> Result<()> {
    let cargo_toml_path = Path::new(expected_name).join("Cargo.toml");
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
        &cargo_toml_path.display()
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

fn get_required_artifact_files(version: &Version) -> Vec<String> {
    let mut required_artifacts = Vec::with_capacity(POSSIBLE_TARGETS.len());
    for target_triple in POSSIBLE_TARGETS {
        required_artifacts.push(format!("supergraph-v{version}-{target_triple}.tar.gz"))
    }
    required_artifacts.push("LICENSE".to_string());
    required_artifacts.push("sha1sums.txt".to_string());
    required_artifacts.push("sha256sums.txt".to_string());
    required_artifacts.push("md5sums.txt".to_string());
    required_artifacts
}

pub(crate) fn assert_includes_required_artifacts(
    version: &Version,
    artifacts_dir: &Path,
) -> Result<()> {
    let required_artifact_files = get_required_artifact_files(version);
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
            artifacts_dir.display(),
            &required_artifact_files
        ));
    }
    if existing_artifact_files.iter().all(|ef| {
        if required_artifact_files.contains(ef) {
           info!("confirmed {} exists", ef);
            true
        } else {
            info!(
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
