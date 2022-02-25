use anyhow::{anyhow, Error, Result};
use semver::Version;

use std::{fmt, str::FromStr};

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

pub(crate) enum Package {
    Harmonizer,
    Supergraph,
    ApolloFederationTypes,
    RouterBridge,
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Package::Harmonizer => "harmonizer",
                Package::Supergraph => "supergraph",
                Package::RouterBridge => "router-bridge",
                Package::ApolloFederationTypes => "apollo-federation-types",
            }
        )
    }
}
