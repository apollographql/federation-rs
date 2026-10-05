#[cfg(feature = "json_schema")]
use schemars::{json_schema, Schema, SchemaGenerator};
use semver::Version;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use serde_with::{DeserializeFromStr, SerializeDisplay};
#[cfg(feature = "json_schema")]
use std::borrow::Cow;
use std::{
    fmt::{self, Display},
    str::FromStr,
};

use crate::config::ConfigError;

pub trait PluginVersion {
    fn get_major_version(&self) -> u64;
    fn get_tarball_version(&self) -> String;
}

#[derive(Debug, Clone, SerializeDisplay, DeserializeFromStr, PartialEq, Eq)]
pub enum RouterVersion {
    Exact(Version),
    LatestOne,
    LatestTwo,
    LatestThree,
}

impl PluginVersion for RouterVersion {
    fn get_major_version(&self) -> u64 {
        match self {
            Self::LatestOne => 1,
            Self::LatestTwo => 2,
            Self::LatestThree => 3,
            Self::Exact(v) => v.major,
        }
    }

    fn get_tarball_version(&self) -> String {
        match self {
            Self::Exact(v) => format!("v{v}"),
            // the endpoint for getting router plugins via rover.apollo.dev
            // uses "latest-plugin" instead of "latest" zsto get the latest version
            Self::LatestOne => "latest-plugin".to_string(),
            Self::LatestTwo => "latest-2".to_string(),
            Self::LatestThree => "latest-3".to_string(),
        }
    }
}

impl Display for RouterVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let result = match self {
            Self::LatestOne => "1".to_string(),
            Self::LatestTwo => "2".to_string(),
            Self::LatestThree => "3".to_string(),
            Self::Exact(version) => format!("={version}"),
        };
        write!(f, "{result}")
    }
}

impl FromStr for RouterVersion {
    type Err = ConfigError;

    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
        let invalid_version = ConfigError::InvalidConfiguration {
            message: format!("Specified version `{input}` is not supported. You can specify '1', '2', '3', 'latest', or a fully qualified version prefixed with an '=', like: =1.0.0"),
        };
        if input.len() > 1 && (input.starts_with('=') || input.starts_with('v')) {
            if let Ok(version) = input[1..].parse::<Version>() {
                Ok(Self::Exact(version))
            } else {
                Err(invalid_version)
            }
        } else {
            match input {
                "1" => Ok(Self::LatestOne),
                "2" | "latest" => Ok(Self::LatestTwo),
                "3" => Ok(Self::LatestThree),
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
    LatestFedThree,
    ExactFedOne(Version),
    ExactFedTwo(Version),
    ExactFedThree(Version),
}

impl FederationVersion {
    pub fn get_exact(&self) -> Option<&Version> {
        match self {
            Self::ExactFedOne(version)
            | Self::ExactFedTwo(version)
            | Self::ExactFedThree(version) => Some(version),
            _ => None,
        }
    }

    fn is_latest(&self) -> bool {
        matches!(
            self,
            Self::LatestFedOne | Self::LatestFedTwo | Self::LatestFedThree
        )
    }

    pub fn is_fed_one(&self) -> bool {
        matches!(self, Self::LatestFedOne) || matches!(self, Self::ExactFedOne(_))
    }

    pub fn is_fed_two(&self) -> bool {
        matches!(self, Self::LatestFedTwo) || matches!(self, Self::ExactFedTwo(_))
    }

    pub fn is_fed_three(&self) -> bool {
        matches!(self, Self::LatestFedThree | Self::ExactFedThree(_))
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
            } else if self.is_fed_three() {
                // all fed3 versions support ARM
                supports_arm = true;
            }
        }
        supports_arm
    }

    pub fn supports_arm_macos(&self) -> bool {
        let mut supports_arm = false;
        // No published fed1 version supports aarch64 on macOS
        if self.is_fed_three() {
            // all fed3 versions support aarch64 on macOS
            supports_arm = true;
        } else if self.is_fed_two() {
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
            Self::LatestFedThree | Self::ExactFedThree(_) => 3,
        }
    }

    fn get_tarball_version(&self) -> String {
        match self {
            Self::LatestFedOne => "latest-0".to_string(),
            Self::LatestFedTwo => "latest-2".to_string(),
            Self::LatestFedThree => "latest-3".to_string(),
            Self::ExactFedOne(v) | Self::ExactFedTwo(v) | Self::ExactFedThree(v) => {
                format!("v{v}")
            }
        }
    }
}

impl Display for FederationVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let result = match self {
            Self::LatestFedOne => "0".to_string(),
            Self::LatestFedTwo => "2".to_string(),
            Self::LatestFedThree => "3".to_string(),
            Self::ExactFedOne(version)
            | Self::ExactFedTwo(version)
            | Self::ExactFedThree(version) => format!("={version}"),
        };
        write!(f, "{result}")
    }
}

impl FromStr for FederationVersion {
    type Err = ConfigError;

    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
        let invalid_version = ConfigError::InvalidConfiguration {
            message: format!("Specified version `{input}` is not supported. You can either specify '1', '2', '3', or a fully qualified version prefixed with an '=', like: =2.0.0"),
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
                } else if version.major == 3 {
                    if version >= "3.0.0-preview.0".parse::<Version>().unwrap() {
                        Ok(Self::ExactFedThree(version))
                    } else {
                        Err(ConfigError::InvalidConfiguration { message: format!("Specified version `{input}` is not supported. The earliest version you can specify for federation 3 is '=3.0.0-preview.0'") })
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
                "3" | "latest-3" => Ok(Self::LatestFedThree),
                _ => Err(invalid_version),
            }
        }
    }
}

#[cfg(feature = "json_schema")]
impl schemars::JsonSchema for FederationVersion {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("FederationVersion")
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "pattern": r#"^(1|2|3|=[23]\.\d+\.\d+.*)$"#
        })
    }
}

impl<'de> Deserialize<'de> for FederationVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = FederationVersion;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("literal '1', '2' or '3' (as a string or number), or a fully qualified version prefixed with an '=', like: =2.0.0")
            }

            fn visit_u64<E>(self, num: u64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                match num {
                    0 | 1 => Ok(FederationVersion::LatestFedOne),
                    2 => Ok(FederationVersion::LatestFedTwo),
                    3 => Ok(FederationVersion::LatestFedThree),
                    _ => Err(Error::custom(format!(
                        "specified version `{num}` is not supported"
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
mod test_router_version {
    use std::str::FromStr;

    use rstest::rstest;
    use semver::Version;

    use super::RouterVersion;

    #[rstest]
    #[case("1", RouterVersion::LatestOne)]
    #[case("2", RouterVersion::LatestTwo)]
    #[case("3", RouterVersion::LatestThree)]
    #[case("latest", RouterVersion::LatestTwo)]
    #[case("=1.0.0", RouterVersion::Exact(Version::new(1, 0, 0)))]
    // Exact pins for majors other than 1 must be accepted (regression: rover#3356).
    #[case("=2.0.0", RouterVersion::Exact(Version::new(2, 0, 0)))]
    #[case("=2.15.0", RouterVersion::Exact(Version::new(2, 15, 0)))]
    #[case("v2.15.0", RouterVersion::Exact(Version::new(2, 15, 0)))]
    #[case("=3.0.0-preview.0", RouterVersion::Exact("3.0.0-preview.0".parse().unwrap()))]
    fn parses_supported_versions(#[case] input: &str, #[case] expected: RouterVersion) {
        assert_eq!(RouterVersion::from_str(input).unwrap(), expected);
    }

    #[rstest]
    #[case("2.15.0")] // missing '=' or 'v' prefix
    #[case("=v2.15.0")] // doubled prefix is not valid semver
    #[case("4")] // not a recognized channel
    #[case("")]
    #[case("garbage")]
    fn rejects_unsupported_versions(#[case] input: &str) {
        assert!(RouterVersion::from_str(input).is_err());
    }

    #[test]
    fn latest_three_plugin_version() {
        use crate::config::PluginVersion;

        let version = RouterVersion::LatestThree;
        assert_eq!(version.get_major_version(), 3);
        assert_eq!(version.get_tarball_version(), "latest-3");
        assert_eq!(version.to_string(), "3");
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

        assert_eq!(
            FederationVersion::LatestFedThree,
            serde_yaml::from_str("3").unwrap()
        );
        assert_eq!(
            FederationVersion::LatestFedThree,
            serde_yaml::from_str("\"3\"").unwrap()
        );
        assert_eq!(
            FederationVersion::LatestFedThree,
            serde_yaml::from_str("latest-3").unwrap()
        );
        assert_eq!(
            FederationVersion::ExactFedThree("3.0.0-preview.0".parse().unwrap()),
            serde_yaml::from_str("=3.0.0-preview.0").unwrap()
        );
        assert_eq!(
            FederationVersion::ExactFedThree("3.1.2".parse().unwrap()),
            serde_yaml::from_str("v3.1.2").unwrap()
        );
    }

    #[rstest]
    #[case("=4.0.0")]
    #[case("=3.0.0-alpha.0")] // earlier than the first fed3 preview
    #[case("4")]
    fn rejects_unsupported_versions(#[case] input: &str) {
        assert!(input.parse::<FederationVersion>().is_err());
    }

    #[rstest]
    #[case(FederationVersion::LatestFedThree, "latest-3", "3")]
    #[case(FederationVersion::ExactFedThree("3.0.0-preview.0".parse().unwrap()), "v3.0.0-preview.0", "=3.0.0-preview.0")]
    fn test_fed_three_plugin_version(
        #[case] version: FederationVersion,
        #[case] tarball: &str,
        #[case] display: &str,
    ) {
        use crate::config::PluginVersion;

        assert_eq!(version.get_major_version(), 3);
        assert_eq!(version.get_tarball_version(), tarball);
        assert_eq!(version.to_string(), display);
        assert!(version.is_fed_three());
        assert!(!version.is_fed_two());
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
    #[case::fed3_latest(FederationVersion::LatestFedThree, true)]
    #[case::fed3_exact(FederationVersion::ExactFedThree("3.0.0-preview.0".parse().unwrap()), true)]
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
    #[case::fed3_latest(FederationVersion::LatestFedThree, true)]
    #[case::fed3_exact(FederationVersion::ExactFedThree("3.0.0-preview.0".parse().unwrap()), true)]
    fn test_supports_arm_macos(#[case] version: FederationVersion, #[case] expected: bool) {
        assert_eq!(version.supports_arm_macos(), expected)
    }
}

#[cfg(feature = "json_schema")]
#[cfg(test)]
mod json_schema_tests {
    use super::*;
    use schemars::{schema_for, JsonSchema, SchemaGenerator};

    #[test]
    fn test_schema_name() {
        assert_eq!(FederationVersion::schema_name(), "FederationVersion");
    }

    #[test]
    fn test_json_schema() {
        let mut gen = SchemaGenerator::default();
        let schema = FederationVersion::json_schema(&mut gen);

        let value = serde_json::to_value(&schema).unwrap();
        assert_eq!(value["pattern"], r#"^(1|2|3|=[23]\.\d+\.\d+.*)$"#);
        // The schema should not have a type field since it's only setting pattern
        assert!(value["type"].is_null());
    }

    #[test]
    fn test_serialize_to_value() {
        let schema = schema_for!(FederationVersion);
        let serialized = serde_json::to_value(&schema).unwrap();

        assert!(serialized.is_object());
        assert_eq!(
            serialized["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(serialized["title"], "FederationVersion");
        assert_eq!(serialized["pattern"], r#"^(1|2|3|=[23]\.\d+\.\d+.*)$"#);
    }

    #[test]
    fn test_serialize_to_json() {
        let schema = schema_for!(FederationVersion);
        let serialized = serde_json::to_string_pretty(&schema).unwrap();

        assert!(
            serialized.contains("\"$schema\": \"https://json-schema.org/draft/2020-12/schema\"")
        );
        assert!(serialized.contains("\"title\": \"FederationVersion\""));
        assert!(serialized.contains("\"pattern\": \"^(1|2|3|=[23]\\\\.\\\\d+\\\\.\\\\d+.*)$\""));
    }
}
