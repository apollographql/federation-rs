// TODO: maybe flesh this out?
// This is possible design for future stuff but doesn't do anything yet.
// rust-analyzer won't even compile it until it's added to ./mod.rs

use std::{collections::BTreeMap, fs, str::FromStr};

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::{Error, Result, SupergraphConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    // Store config in a BTreeMap as HashMap is non-deterministic.
    supergraphs: BTreeMap<String, SupergraphConfig>,

    // Default configuration options for the project.
    default: Option<ProjectDefault>,

    // Platform version for the project.
    version: Option<PlatformVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDefault {
    supergraph: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlatformVersion {
    Alpha,
}

impl FromStr for PlatformVersion {
    type Err = Error;

    fn from_str(version: &str) -> std::result::Result<Self, Self::Err> {
        match version {
            "alpha" => Ok(PlatformVersion::Alpha),
            _ => Err(Error::InvalidConfiguration {
                message: format!("Specified version `{}` is not supported", version),
            }),
        }
    }
}

impl ProjectConfig {
    /// Create a new ProjectConfig
    pub fn new(
        input: &[(String, SupergraphConfig)],
        version: Option<PlatformVersion>,
        default: Option<ProjectDefault>,
    ) -> Self {
        let mut supergraphs = BTreeMap::new();

        for (name, config) in input {
            supergraphs.insert(name.to_string(), config.to_owned());
        }

        ProjectConfig {
            supergraphs,
            version,
            default,
        }
    }

    /// Create a new ProjectConfig from a YAML string in memory.
    pub fn new_from_yaml(yaml: &str) -> Result<ProjectConfig> {
        let parsed_config =
            serde_yaml::from_str(yaml).map_err(|e| Error::InvalidConfiguration {
                message: e.to_string(),
            })?;

        log::debug!("{:?}", parsed_config);

        Ok(parsed_config)
    }

    /// Create a new ProjectConfig from a YAML file.
    pub fn new_from_yaml_file<P: Into<Utf8PathBuf>>(config_path: P) -> Result<ProjectConfig> {
        let config_path: Utf8PathBuf = config_path.into();
        let project_yaml = fs::read_to_string(&config_path).map_err(|e| Error::MissingFile {
            file_path: config_path.to_string(),
            message: e.to_string(),
        })?;

        let parsed_config = ProjectConfig::new_from_yaml(&project_yaml)?;

        Ok(parsed_config)
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectConfig;

    #[test]
    fn it_can_parse_valid_config() {
        let raw_good_yaml = r#"version: alpha
  default:
    supergraph: averys-graph
  supergraphs:
    averys-supergraph:
      default:
        variant: current
        endpoint:
          local: http://localhost:4000/graphql
          studio: https://api.avery.dev/graphql
      subgraphs:
        local-subgraph:
          command: npm run start:subgraph-1
          path: ./subgraphs/subgraph-1.graphql
        introspected-subgraph: # rover subgraph introspect
          command: npm run start:subgraph-2
          endpoint: http://localhost:4001/graphql
        monorepo-monolith: # rover graph introspect
          graph:
            name: averys-legacy-monolith
        studio-graph-with-all-subgraph: # rover supergraph fetch
          graph:
            name: big-graph
            variant: mock
        studio-graph-with-one-subgraph:
          graph:
            name: other-supergraph
            variant: dev
            subgraph: subgraph-two
        introspected-graph:
          endpoint: http://localhost:4002/graphql
    averys-legacy-monolith:
      default-variant: prod
      endpoint:
        local: http://localhost:3999/graphql
        deployed: https://api.avery.dev/legacy/graphql
      subgraph:
        only-subgraph:
          command: php artisan serve --port 9090
          schema:
            endpoint: http://localhost:9090
"#;

        assert!(ProjectConfig::new_from_yaml(raw_good_yaml).is_ok())
    }
}
