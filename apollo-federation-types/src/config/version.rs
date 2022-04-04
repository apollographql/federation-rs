use crate::config::ConfigError;

use serde::{Deserialize, Serialize};

use std::{
    fmt::{self, Display},
    str::FromStr,
};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum FederationVersion {
    FedOne,
    FedTwo,
}

impl Default for FederationVersion {
    fn default() -> Self {
        FederationVersion::FedTwo
    }
}

impl Display for FederationVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let result = match self {
            Self::FedOne => "0",
            Self::FedTwo => "2",
        };
        write!(f, "{}", result)
    }
}

impl FromStr for FederationVersion {
    type Err = ConfigError;

    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
        match input {
            "0" | "1" => Ok(Self::FedOne),
            "2" => Ok(Self::FedTwo),
            _ => Err(ConfigError::InvalidConfiguration {
                message: format!("Specified version `{}` is not supported", input),
            }),
        }
    }
}
