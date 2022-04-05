use crate::config::ConfigError;

use semver::Version;
use serde::{Deserialize, Serialize};

use std::{
    fmt::{self, Display},
    str::FromStr,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FederationVersion {
    LatestFedOne,
    LatestFedTwo,
    ExactFedOne(Version),
    ExactFedTwo(Version),
}

impl FederationVersion {
    pub fn get_major_version(&self) -> u64 {
        match self {
            Self::LatestFedOne | Self::ExactFedOne(_) => 0,
            Self::LatestFedTwo | Self::ExactFedTwo(_) => 2,
        }
    }

    pub fn get_exact(&self) -> Option<&Version> {
        match self {
            Self::ExactFedOne(version) | Self::ExactFedTwo(version) => Some(version),
            _ => None,
        }
    }

    pub fn is_fed_one(&self) -> bool {
        matches!(self, Self::LatestFedOne) || matches!(self, Self::ExactFedOne(_))
    }

    pub fn is_fed_two(&self) -> bool {
        matches!(self, Self::LatestFedTwo) || matches!(self, Self::ExactFedTwo(_))
    }

    pub fn get_tarball_version(&self) -> String {
        match self {
            Self::LatestFedOne => "latest-0".to_string(),
            Self::LatestFedTwo => "latest-2".to_string(),
            Self::ExactFedOne(v) | Self::ExactFedTwo(v) => format!("v{}", v),
        }
    }
}

impl Default for FederationVersion {
    fn default() -> Self {
        FederationVersion::LatestFedOne
    }
}

impl Display for FederationVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let result = match self {
            Self::LatestFedOne => "0".to_string(),
            Self::LatestFedTwo => "2".to_string(),
            Self::ExactFedOne(version) | Self::ExactFedTwo(version) => format!("={}", version),
        };
        write!(f, "{}", result)
    }
}

impl FromStr for FederationVersion {
    type Err = ConfigError;

    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
        let invalid_version = ConfigError::InvalidConfiguration {
            message: format!("Specified version `{}` is not supported. You can either specify '1', '2', or a fully qualified version prefixed with an '=', like: =2.0.0", input),
        };
        if input.len() > 1 && (input.starts_with('=') || input.starts_with('v')) {
            if let Ok(version) = input[1..].parse::<Version>() {
                if version.major == 0 {
                    if version.minor >= 36 {
                        Ok(Self::ExactFedOne(version))
                    } else {
                        Err(ConfigError::InvalidConfiguration { message: format!("Specified version `{}` is not supported. The earliest version you can specify for federation 1 is '=0.36.0'", input) })
                    }
                } else if version.major == 2 {
                    if version >= "2.0.0-preview.9".parse::<Version>().unwrap() {
                        Ok(Self::ExactFedTwo(version))
                    } else {
                        Err(ConfigError::InvalidConfiguration { message: format!("Specified version `{}` is not supported. The earliest version you can specify for federation 2 is '=2.0.0-preview.9'", input) })
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
