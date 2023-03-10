use crate::config::ConfigError;

use semver::Version;
use serde_with::{DeserializeFromStr, SerializeDisplay};

use std::{
    fmt::{self, Display},
    str::FromStr,
};

pub trait PluginVersion {
    fn get_major_version(&self) -> u64;
    fn get_tarball_version(&self) -> String;
}

#[derive(Debug, Clone, SerializeDisplay, DeserializeFromStr, PartialEq, Eq)]
pub enum RouterVersion {
    Exact(Version),
    Latest,
}

impl PluginVersion for RouterVersion {
    fn get_major_version(&self) -> u64 {
        match self {
            Self::Latest => 1,
            Self::Exact(v) => v.major,
        }
    }

    fn get_tarball_version(&self) -> String {
        match self {
            Self::Exact(v) => format!("v{v}"),
            // the endpoint for getting router plugins via rover.apollo.dev
            // uses "latest-plugin" instead of "latest" zsto get the latest version
            Self::Latest => "latest-plugin".to_string(),
        }
    }
}

impl Display for RouterVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let result = match self {
            Self::Latest => "1".to_string(),
            Self::Exact(version) => format!("={version}"),
        };
        write!(f, "{result}")
    }
}

impl FromStr for RouterVersion {
    type Err = ConfigError;

    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
        let invalid_version = ConfigError::InvalidConfiguration {
            message: format!("Specified version `{input}` is not supported. You can either specify '1', 'latest', or a fully qualified version prefixed with an '=', like: =1.0.0"),
        };
        if input.len() > 1 && (input.starts_with('=') || input.starts_with('v')) {
            if let Ok(version) = input[1..].parse::<Version>() {
                if version.major == 1 {
                    Ok(Self::Exact(version))
                } else {
                    Err(invalid_version)
                }
            } else {
                Err(invalid_version)
            }
        } else {
            match input {
                "1" | "latest" => Ok(Self::Latest),
                _ => Err(invalid_version),
            }
        }
    }
}

#[derive(Debug, Clone, DeserializeFromStr, SerializeDisplay, Eq, PartialEq, Default)]
pub enum FederationVersion {
    #[default]
    LatestFedOne,
    LatestFedTwo,
    ExactFedOne(Version),
    ExactFedTwo(Version),
}

impl FederationVersion {
    pub fn get_exact(&self) -> Option<&Version> {
        match self {
            Self::ExactFedOne(version) | Self::ExactFedTwo(version) => Some(version),
            _ => None,
        }
    }

    fn is_latest(&self) -> bool {
        matches!(self, Self::LatestFedOne) || matches!(self, Self::LatestFedTwo)
    }

    pub fn is_fed_one(&self) -> bool {
        matches!(self, Self::LatestFedOne) || matches!(self, Self::ExactFedOne(_))
    }

    pub fn is_fed_two(&self) -> bool {
        matches!(self, Self::LatestFedTwo) || matches!(self, Self::ExactFedTwo(_))
    }

    pub fn supports_arm_linux(&self) -> bool {
        let mut supports_arm = false;
        if self.is_latest() {
            supports_arm = true;
        } else if let Some(exact) = self.get_exact() {
            if self.is_fed_one() {
                // 0.37.0 is the first fed2 version that supports ARM
                supports_arm = exact.minor >= 37;
            } else if self.is_fed_two() {
                // 2.1.0 is the first fed2 version that supports ARM
                supports_arm = exact.minor >= 1;
            }
        }
        supports_arm
    }
}

impl PluginVersion for FederationVersion {
    fn get_major_version(&self) -> u64 {
        match self {
            Self::LatestFedOne | Self::ExactFedOne(_) => 0,
            Self::LatestFedTwo | Self::ExactFedTwo(_) => 2,
        }
    }

    fn get_tarball_version(&self) -> String {
        match self {
            Self::LatestFedOne => "latest-0".to_string(),
            Self::LatestFedTwo => "latest-2".to_string(),
            Self::ExactFedOne(v) | Self::ExactFedTwo(v) => format!("v{v}"),
        }
    }
}

impl Display for FederationVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let result = match self {
            Self::LatestFedOne => "0".to_string(),
            Self::LatestFedTwo => "2".to_string(),
            Self::ExactFedOne(version) | Self::ExactFedTwo(version) => format!("={version}"),
        };
        write!(f, "{result}")
    }
}

impl FromStr for FederationVersion {
    type Err = ConfigError;

    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
        let invalid_version = ConfigError::InvalidConfiguration {
            message: format!("Specified version `{input}` is not supported. You can either specify '1', '2', or a fully qualified version prefixed with an '=', like: =2.0.0"),
        };
        if input.len() > 1 && (input.starts_with('=') || input.starts_with('v')) {
            if let Ok(version) = input[1..].parse::<Version>() {
                if version.major == 0 {
                    if version.minor >= 36 {
                        Ok(Self::ExactFedOne(version))
                    } else {
                        Err(ConfigError::InvalidConfiguration { message: format!("Specified version `{input}` is not supported. The earliest version you can specify for federation 1 is '=0.36.0'") })
                    }
                } else if version.major == 2 {
                    if version >= "2.0.0-preview.9".parse::<Version>().unwrap() {
                        Ok(Self::ExactFedTwo(version))
                    } else {
                        Err(ConfigError::InvalidConfiguration { message: format!("Specified version `{input}` is not supported. The earliest version you can specify for federation 2 is '=2.0.0-preview.9'") })
                    }
                } else {
                    Err(invalid_version)
                }
            } else {
                Err(invalid_version)
            }
        } else {
            match input {
                "0" | "1" | "latest-0" | "latest-1" => Ok(Self::LatestFedOne),
                "2" | "latest-2" => Ok(Self::LatestFedTwo),
                _ => Err(invalid_version),
            }
        }
    }
}
