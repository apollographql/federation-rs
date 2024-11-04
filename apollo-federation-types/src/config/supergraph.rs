use std::{collections::BTreeMap, fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    config::{ConfigError, ConfigResult, FederationVersion, SubgraphConfig},
    javascript::SubgraphDefinition,
};

/// The configuration for a single supergraph
/// composed of multiple subgraphs.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "json_schema", derive(schemars::JsonSchema))]
pub struct SupergraphConfig {
    // Store config in a BTreeMap, as HashMap is non-deterministic.
    subgraphs: BTreeMap<String, SubgraphConfig>,

    // The version requirement for the supergraph binary.
    federation_version: Option<FederationVersion>,
}

impl SupergraphConfig {
    /// Creates a new SupergraphConfig
    pub fn new(
        subgraphs: BTreeMap<String, SubgraphConfig>,
        federation_version: Option<FederationVersion>,
    ) -> SupergraphConfig {
        SupergraphConfig {
            subgraphs,
            federation_version,
        }
    }
    /// Create a new SupergraphConfig from a YAML string in memory.
    pub fn new_from_yaml(yaml: &str) -> ConfigResult<SupergraphConfig> {
        let parsed_config: SupergraphConfig =
            serde_yaml::from_str(yaml).map_err(|e| ConfigError::InvalidConfiguration {
                message: e.to_string(),
            })?;

        log::debug!("{:?}", parsed_config);

        Ok(parsed_config)
    }

    /// Create a new SupergraphConfig from a JSON string in memory.
    pub fn new_from_json(json: &str) -> ConfigResult<SupergraphConfig> {
        let parsed_config: SupergraphConfig =
            serde_json::from_str(json).map_err(|e| ConfigError::InvalidConfiguration {
                message: e.to_string(),
            })?;

        log::debug!("{:?}", parsed_config);

        Ok(parsed_config)
    }

    /// Create a new SupergraphConfig from a YAML file.
    pub fn new_from_yaml_file<P: Into<PathBuf>>(config_path: P) -> ConfigResult<SupergraphConfig> {
        let config_path: PathBuf = config_path.into();
        let supergraph_yaml =
            fs::read_to_string(&config_path).map_err(|e| ConfigError::MissingFile {
                file_path: config_path.display().to_string(),
                message: e.to_string(),
            })?;

        let parsed_config = SupergraphConfig::new_from_yaml(&supergraph_yaml)?;

        Ok(parsed_config)
    }

    /// Returns a Vec of resolved subgraphs, if and only if they are all resolved.
    /// Resolved in this sense means that each subgraph config includes
    /// a name, a URL, and raw SDL.
    pub fn get_subgraph_definitions(&self) -> ConfigResult<Vec<SubgraphDefinition>> {
        let mut subgraph_definitions = Vec::new();
        let mut unresolved_subgraphs = Vec::new();
        for (subgraph_name, subgraph_config) in &self.subgraphs {
            if let Some(sdl) = subgraph_config.get_sdl() {
                if let Some(routing_url) = &subgraph_config.routing_url {
                    subgraph_definitions.push(SubgraphDefinition {
                        name: subgraph_name.clone(),
                        url: routing_url.clone(),
                        sdl,
                    });
                } else {
                    unresolved_subgraphs.push(subgraph_name);
                }
            } else {
                unresolved_subgraphs.push(subgraph_name);
            }
        }
        if !unresolved_subgraphs.is_empty() {
            Err(ConfigError::SubgraphsNotResolved {
                subgraph_names: format!("{:?}", &unresolved_subgraphs),
            })
        } else if subgraph_definitions.is_empty() {
            Err(ConfigError::NoSubgraphsFound)
        } else {
            Ok(subgraph_definitions)
        }
    }

    /// Updates the federation_version for a configuration
    pub fn set_federation_version(&mut self, federation_version: FederationVersion) {
        self.federation_version = Some(federation_version);
    }

    /// Gets the current federation_version for a configuration
    pub fn get_federation_version(&self) -> Option<FederationVersion> {
        self.federation_version.clone()
    }

    /// Merges the subgraphs of another [`SupergraphConfig`] into this one; the
    /// other config takes precedence when there are overlaps
    pub fn merge_subgraphs(&mut self, other: &SupergraphConfig) {
        for (key, other_subgraph) in other.subgraphs.iter() {
            let other_subgraph = other_subgraph.clone();
            // SubgraphConfig always has a schema. For routing_url, we take
            // `other` if they both exist (ie, we let local configuration
            // override)
            let merged_subgraph = match self.subgraphs.get(key) {
                Some(my_subgraph) => SubgraphConfig {
                    routing_url: other_subgraph
                        .routing_url
                        .or(my_subgraph.routing_url.clone()),
                    schema: other_subgraph.schema,
                },
                None => other_subgraph,
            };
            self.subgraphs.insert(key.to_string(), merged_subgraph);
        }
    }
}

impl From<Vec<SubgraphDefinition>> for SupergraphConfig {
    fn from(input: Vec<SubgraphDefinition>) -> Self {
        let mut subgraphs = BTreeMap::new();
        for subgraph_definition in input {
            subgraphs.insert(
                subgraph_definition.name,
                SubgraphConfig {
                    routing_url: Some(subgraph_definition.url),
                    schema: crate::config::SchemaSource::Sdl {
                        sdl: subgraph_definition.sdl,
                    },
                },
            );
        }
        Self {
            subgraphs,
            federation_version: None,
        }
    }
}

// implement IntoIterator so you can do:
// for (subgraph_name, subgraph_metadata) in supergraph_config.into_iter() { ... }
impl IntoIterator for SupergraphConfig {
    type Item = (String, SubgraphConfig);
    type IntoIter = std::collections::btree_map::IntoIter<String, SubgraphConfig>;

    fn into_iter(self) -> Self::IntoIter {
        self.subgraphs.into_iter()
    }
}

impl FromIterator<(String, SubgraphConfig)> for SupergraphConfig {
    fn from_iter<T: IntoIterator<Item = (String, SubgraphConfig)>>(iter: T) -> Self {
        Self {
            subgraphs: iter.into_iter().collect::<BTreeMap<_, _>>(),
            federation_version: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, convert::TryFrom, fs, path::PathBuf};

    use assert_fs::TempDir;
    use semver::Version;

    use super::SupergraphConfig;
    use crate::config::{FederationVersion, SchemaSource, SubgraphConfig};

    #[test]
    fn it_can_parse_valid_config_without_version() {
        let raw_good_yaml = r#"---
subgraphs:
  films:
    routing_url: https://films.example.com
    schema:
      file: ./good-films.graphql
  people:
    routing_url: https://people.example.com
    schema:
      file: ./good-people.graphql
"#;

        let config = SupergraphConfig::new_from_yaml(raw_good_yaml);
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.federation_version, None);
    }

    #[test]
    fn it_can_parse_valid_config_without_version_json() {
        let raw_good_json = r#"
{
    "subgraphs": {
      "films": {
        "routing_url": "https://films.example.com",
        "schema": {
          "file": "./good-films.graphql"
        }
      },
      "people": {
        "routing_url": "https://people.example.com",
        "schema": {
          "file": "./good-people.graphql"
        }
      }
    }
  }
"#;

        let config = SupergraphConfig::new_from_json(raw_good_json);
        println!("{:?}", config);
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.federation_version, None);
    }

    #[test]
    fn it_can_parse_valid_config_fed_zero() {
        let raw_good_yaml = r#"---
federation_version: 0
subgraphs:
  films:
    routing_url: https://films.example.com
    schema:
      file: ./good-films.graphql
  people:
    routing_url: https://people.example.com
    schema:
      file: ./good-people.graphql
"#;

        let config = SupergraphConfig::new_from_yaml(raw_good_yaml).unwrap();
        assert_eq!(
            config.federation_version,
            Some(FederationVersion::LatestFedOne)
        );
    }

    #[test]
    fn it_can_parse_valid_config_fed_zero_json() {
        let raw_json_yaml = r#"
{
    "federation_version": 0,
    "subgraphs": {
      "films": {
        "routing_url": "https://films.example.com",
        "schema": {
          "file": "./good-films.graphql"
        }
      },
      "people": {
        "routing_url": "https://people.example.com",
        "schema": {
          "file": "./good-people.graphql"
        }
      }
    }
  }
"#;

        let config = SupergraphConfig::new_from_json(raw_json_yaml).unwrap();
        assert_eq!(
            config.federation_version,
            Some(FederationVersion::LatestFedOne)
        );
    }

    #[test]
    fn it_can_parse_valid_config_fed_one() {
        let raw_good_yaml = r#"---
federation_version: 1
subgraphs:
  films:
    routing_url: https://films.example.com
    schema:
      file: ./good-films.graphql
  people:
    routing_url: https://people.example.com
    schema:
      file: ./good-people.graphql
"#;

        let config = SupergraphConfig::new_from_yaml(raw_good_yaml).unwrap();
        assert_eq!(
            config.federation_version,
            Some(FederationVersion::LatestFedOne)
        );
    }

    #[test]
    fn it_can_parse_valid_config_fed_one_json() {
        let raw_good_json = r#"
{
    "federation_version": 1,
    "subgraphs": {
      "films": {
        "routing_url": "https://films.example.com",
        "schema": {
          "file": "./good-films.graphql"
        }
      },
      "people": {
        "routing_url": "https://people.example.com",
        "schema": {
          "file": "./good-people.graphql"
        }
      }
    }
  }
"#;

        let config = SupergraphConfig::new_from_json(raw_good_json).unwrap();
        assert_eq!(
            config.federation_version,
            Some(FederationVersion::LatestFedOne)
        );
    }

    #[test]
    fn it_can_parse_valid_config_fed_two() {
        let raw_good_yaml = r#"---
federation_version: 2
subgraphs:
  films:
    routing_url: https://films.example.com
    schema:
      file: ./good-films.graphql
  people:
    routing_url: https://people.example.com
    schema:
      file: ./good-people.graphql
"#;

        let config = SupergraphConfig::new_from_yaml(raw_good_yaml).unwrap();
        assert_eq!(
            config.federation_version,
            Some(FederationVersion::LatestFedTwo)
        );
    }

    #[test]
    fn it_can_parse_valid_config_fed_two_json() {
        let raw_good_json = r#"
{
    "federation_version": 2,
    "subgraphs": {
      "films": {
        "routing_url": "https://films.example.com",
        "schema": {
          "file": "./good-films.graphql"
        }
      },
      "people": {
        "routing_url": "https://people.example.com",
        "schema": {
          "file": "./good-people.graphql"
        }
      }
    }
  }
"#;

        let config = SupergraphConfig::new_from_json(raw_good_json).unwrap();
        assert_eq!(
            config.federation_version,
            Some(FederationVersion::LatestFedTwo)
        );
    }

    #[test]
    fn it_can_parse_valid_config_fed_one_exact() {
        let raw_good_yaml = r#"---
federation_version: =0.36.0
subgraphs:
  films:
    routing_url: https://films.example.com
    schema:
      file: ./good-films.graphql
  people:
    routing_url: https://people.example.com
    schema:
      file: ./good-people.graphql
"#;

        let config = SupergraphConfig::new_from_yaml(raw_good_yaml).unwrap();
        assert_eq!(
            config.federation_version,
            Some(FederationVersion::ExactFedOne(
                Version::parse("0.36.0").unwrap()
            ))
        );
    }

    #[test]
    fn it_can_parse_valid_config_fed_one_exact_json() {
        let raw_good_json = r#"
{
    "federation_version": "=0.36.0",
    "subgraphs": {
      "films": {
        "routing_url": "https://films.example.com",
        "schema": {
          "file": "./good-films.graphql"
        }
      },
      "people": {
        "routing_url": "https://people.example.com",
        "schema": {
          "file": "./good-people.graphql"
        }
      }
    }
  }
"#;

        let config = SupergraphConfig::new_from_json(raw_good_json).unwrap();
        assert_eq!(
            config.federation_version,
            Some(FederationVersion::ExactFedOne(
                Version::parse("0.36.0").unwrap()
            ))
        );
    }

    #[test]
    fn it_can_parse_valid_config_fed_two_exact() {
        let raw_good_yaml = r#"---
federation_version: =2.0.0
subgraphs:
  films:
    routing_url: https://films.example.com
    schema:
      file: ./good-films.graphql
  people:
    routing_url: https://people.example.com
    schema:
      file: ./good-people.graphql
"#;

        let config = SupergraphConfig::new_from_yaml(raw_good_yaml).unwrap();
        assert_eq!(
            config.federation_version,
            Some(FederationVersion::ExactFedTwo(
                Version::parse("2.0.0").unwrap()
            ))
        );
    }

    #[test]
    fn it_can_parse_valid_config_fed_two_exact_json() {
        let raw_good_json = r#"
{
    "federation_version": "=2.0.0",
    "subgraphs": {
      "films": {
        "routing_url": "https://films.example.com",
        "schema": {
          "file": "./good-films.graphql"
        }
      },
      "people": {
        "routing_url": "https://people.example.com",
        "schema": {
          "file": "./good-people.graphql"
        }
      }
    }
  }
"#;

        let config = SupergraphConfig::new_from_json(raw_good_json).unwrap();
        assert_eq!(
            config.federation_version,
            Some(FederationVersion::ExactFedTwo(
                Version::parse("2.0.0").unwrap()
            ))
        );
    }

    #[test]
    fn it_can_parse_valid_config_from_fs() {
        let raw_good_yaml = r#"---
subgraphs:
  films:
    routing_url: https://films.example.com
    schema:
      file: ./good-films.graphql
  people:
    routing_url: https://people.example.com
    schema:
      file: ./good-people.graphql
"#;

        let tmp_home = TempDir::new().unwrap();
        let mut config_path = PathBuf::try_from(tmp_home.path().to_path_buf()).unwrap();
        config_path.push("config.yaml");
        fs::write(&config_path, raw_good_yaml).unwrap();

        assert!(SupergraphConfig::new_from_yaml_file(&config_path).is_ok());
    }

    #[test]
    fn it_can_parse_valid_config_with_introspection() {
        let raw_good_yaml = r#"---
subgraphs:
  films:
    routing_url: https://films.example.com
    schema:
      file: ./films.graphql
  people:
    schema:
      subgraph_url: https://people.example.com
  reviews:
    schema:
      graphref: mygraph@current
      subgraph: reviews
"#;

        assert!(SupergraphConfig::new_from_yaml(raw_good_yaml).is_ok());
    }

    #[test]
    fn it_can_parse_valid_config_with_introspection_json() {
        let raw_good_json = r#"
{
    "subgraphs": {
      "films": {
        "routing_url": "https://films.example.com",
        "schema": {
          "file": "./films.graphql"
        }
      },
      "people": {
        "schema": {
          "subgraph_url": "https://people.example.com"
        }
      },
      "reviews": {
        "schema": {
          "graphref": "mygraph@current",
          "subgraph": "reviews"
        }
      }
    }
  }
"#;

        assert!(SupergraphConfig::new_from_json(raw_good_json).is_ok());
    }

    #[test]
    fn it_errors_on_invalid_config() {
        let raw_bad_yaml = r#"---
subgraphs:
  films:
    routing_______url: https://films.example.com
    schemaaaa:
        file:: ./good-films.graphql
  people:
    routing____url: https://people.example.com
    schema_____file: ./good-people.graphql"#;

        assert!(SupergraphConfig::new_from_yaml(raw_bad_yaml).is_err())
    }

    #[test]
    fn it_errors_on_invalid_config_json() {
        let raw_bad_yaml = r#"
subgraphs:
  films:
    routing_______url: https://films.example.com
    schemaaaa:
        file:: ./good-films.graphql
  people:
    routing____url: https://people.example.com
    schema_____file: ./good-people.graphql"#;

        assert!(SupergraphConfig::new_from_yaml(raw_bad_yaml).is_err())
    }

    #[test]
    fn it_errs_on_bad_version() {
        let raw_good_yaml = r#"---
federation_version: 3"
subgraphs:
  films:
    routing_url: https://films.example.com
    schema:
      file: ./good-films.graphql
  people:
    routing_url: https://people.example.com
    schema:
      file: ./good-people.graphql
"#;

        assert!(SupergraphConfig::new_from_yaml(raw_good_yaml).is_err())
    }

    #[test]
    fn it_errs_on_bad_version_json() {
        let raw_good_yaml = r#"
{
    "federation_version": "3",
    "subgraphs": {
      "films": {
        "routing_url": "https://films.example.com",
        "schema": {
          "file": "./good-films.graphql"
        }
      },
      "people": {
        "routing_url": "https://people.example.com",
        "schema": {
          "file": "./good-people.graphql"
        }
      }
    }
  }
"#;

        assert!(SupergraphConfig::new_from_yaml(raw_good_yaml).is_err())
    }

    #[test]
    fn test_merge_subgraphs() {
        let raw_base_config = r#"---
federation_version: 2
subgraphs:
  films:
    routing_url: https://films.example.com
    schema:
      file: ./good-films.graphql
  people:
    routing_url: https://people.example.com
    schema:
      file: ./good-people.graphql
  robots:
    routing_url: https://robots.example.com
    schema:
      file: ./good-robots.graphql
"#;
        let raw_override_config = r#"---
federation_version: 1
subgraphs:
  films:
    routing_url: https://films.example.com/graphql
    schema:
      file: ./good-films.graphql
  books:
    routing_url: https://books.example.com
    schema:
      file: ./good-books.graphql
  robots:
    schema:
      file: ./better-robots.graphql
"#;
        let mut base_config = SupergraphConfig::new_from_yaml(raw_base_config)
            .expect("Failed to parse supergraph config");

        let override_config = SupergraphConfig::new_from_yaml(raw_override_config)
            .expect("Failed to parse supergraph config");

        base_config.merge_subgraphs(&override_config);

        assert_eq!(
            base_config.get_federation_version(),
            Some(FederationVersion::LatestFedTwo)
        );

        let expected_subgraphs = BTreeMap::from([
            (
                "films".to_string(),
                SubgraphConfig {
                    routing_url: Some("https://films.example.com/graphql".to_string()),
                    schema: SchemaSource::File {
                        file: "./good-films.graphql".into(),
                    },
                },
            ),
            (
                "books".to_string(),
                SubgraphConfig {
                    routing_url: Some("https://books.example.com".to_string()),
                    schema: SchemaSource::File {
                        file: "./good-books.graphql".into(),
                    },
                },
            ),
            (
                "people".to_string(),
                SubgraphConfig {
                    routing_url: Some("https://people.example.com".to_string()),
                    schema: SchemaSource::File {
                        file: "./good-people.graphql".into(),
                    },
                },
            ),
            (
                "robots".to_string(),
                SubgraphConfig {
                    routing_url: Some("https://robots.example.com".to_string()),
                    schema: SchemaSource::File {
                        file: "./better-robots.graphql".into(),
                    },
                },
            ),
        ]);

        assert_eq!(base_config.subgraphs, expected_subgraphs);
    }

    #[test]
    fn test_supergraph_config_from_iterator() {
        let iter = [(
            "subgraph_tmp".to_string(),
            SubgraphConfig {
                routing_url: Some("url".to_string()),
                schema: SchemaSource::Sdl {
                    sdl: "subgraph_tmp".to_string(),
                },
            },
        )]
        .into_iter();

        let s: SupergraphConfig = iter.collect();
        assert_eq!(None, s.get_federation_version());
        assert!(s.get_subgraph_definitions().is_ok());
        assert_eq!(1, s.get_subgraph_definitions().unwrap().len());
    }
}
