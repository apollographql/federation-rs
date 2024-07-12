use std::{
    fmt::{self, Display},
    str::FromStr,
};

use semver::Version;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::config::ConfigError;

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

#[derive(Debug, Clone, SerializeDisplay, Eq, PartialEq, Default)]
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

    pub fn supports_arm_macos(&self) -> bool {
        let mut supports_arm = false;
        // No published fed1 version supports aarch64 on macOS
        if self.is_fed_two() {
            if self.is_latest() {
                supports_arm = true;
            } else if let Some(exact) = self.get_exact() {
                    // v2.7.3 is the earliest version published with aarch64 support for macOS
                    supports_arm = exact.ge(&Version::parse("2.7.3").unwrap())
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

impl<'de> Deserialize<'de> for FederationVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = FederationVersion;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("literal '1' or '2' (as a string or number), or a fully qualified version prefixed with an '=', like: =2.0.0")
            }

            fn visit_u64<E>(self, num: u64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                match num {
                    0 | 1 => Ok(FederationVersion::LatestFedOne),
                    2 => Ok(FederationVersion::LatestFedTwo),
                    _ => Err(Error::custom(format!(
                        "specified version `{}` is not supported",
                        num
                    ))),
                }
            }

            fn visit_str<E>(self, id: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                FederationVersion::from_str(id).map_err(|e| Error::custom(e.to_string()))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

#[cfg(test)]
mod test_federation_version {
    use rstest::rstest;
    use serde_yaml::Value;

    use crate::config::FederationVersion;

    #[test]
    fn test_deserialization() {
        assert_eq!(
            FederationVersion::LatestFedTwo,
            serde_yaml::from_value(Value::String(String::from("2"))).unwrap()
        );
        assert_eq!(
            FederationVersion::LatestFedTwo,
            serde_yaml::from_value(Value::Number(2.into())).unwrap()
        );
        assert_eq!(
            FederationVersion::LatestFedTwo,
            serde_yaml::from_str("latest-2").unwrap()
        );

        assert_eq!(
            FederationVersion::LatestFedOne,
            serde_yaml::from_str("1").unwrap()
        );
        assert_eq!(
            FederationVersion::LatestFedOne,
            serde_yaml::from_str("\"1\"").unwrap()
        );
        assert_eq!(
            FederationVersion::LatestFedOne,
            serde_yaml::from_str("latest-1").unwrap()
        );
        assert_eq!(
            FederationVersion::LatestFedOne,
            serde_yaml::from_str("latest-0").unwrap()
        );

        assert_eq!(
            FederationVersion::ExactFedTwo("2.3.4".parse().unwrap()),
            serde_yaml::from_str("=2.3.4").unwrap()
        );
        assert_eq!(
            FederationVersion::ExactFedTwo("2.3.4".parse().unwrap()),
            serde_yaml::from_str("v2.3.4").unwrap()
        );

        assert_eq!(
            FederationVersion::ExactFedOne("0.37.8".parse().unwrap()),
            serde_yaml::from_str("=0.37.8").unwrap()
        );
        assert_eq!(
            FederationVersion::ExactFedOne("0.37.8".parse().unwrap()),
            serde_yaml::from_str("v0.37.8").unwrap()
        );
    }

    #[rstest]
    #[case::fed1_latest(FederationVersion::LatestFedOne, true)]
    #[case::fed1_supported(FederationVersion::ExactFedOne("0.37.2".parse().unwrap()), true)]
    #[case::fed1_supported_boundary(FederationVersion::ExactFedOne("0.37.1".parse().unwrap()), true)]
    #[case::fed1_unsupported(FederationVersion::ExactFedOne("0.25.0".parse().unwrap()), false)]
    #[case::fed2_latest(FederationVersion::LatestFedTwo, true)]
    #[case::fed2_supported(FederationVersion::ExactFedTwo("2.4.5".parse().unwrap()), true)]
    #[case::fed2_supported_boundary(FederationVersion::ExactFedTwo("2.1.0".parse().unwrap()), true)]
    #[case::fed2_unsupported(FederationVersion::ExactFedTwo("2.0.1".parse().unwrap()), false)]
    fn test_supports_arm_linux(#[case] version: FederationVersion, #[case] expected: bool) {
        assert_eq!(version.supports_arm_linux(), expected)
    }

    #[rstest]
    #[case::fed1_latest(FederationVersion::LatestFedOne, false)]
    #[case::fed1_unsupported(FederationVersion::ExactFedOne("0.37.2".parse().unwrap()), false)]
    #[case::fed2_latest(FederationVersion::LatestFedTwo, true)]
    #[case::fed2_supported(FederationVersion::ExactFedTwo("2.8.1".parse().unwrap()), true)]
    #[case::fed2_supported_boundary(FederationVersion::ExactFedTwo("2.7.3".parse().unwrap()), true)]
    #[case::fed2_unsupported(FederationVersion::ExactFedTwo("2.6.5".parse().unwrap()), false)]
    fn test_supports_arm_macos(#[case] version: FederationVersion, #[case] expected: bool) {
        assert_eq!(version.supports_arm_macos(), expected)
    }
}
