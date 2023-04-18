use std::io::Read;

use camino::Utf8PathBuf;
use structopt::StructOpt;

use apollo_federation_types::{
    build::BuildResult,
    config::{ConfigError, PluginVersion, SupergraphConfig},
};
use harmonizer::harmonize;

#[derive(Debug, StructOpt)]
pub struct Compose {
    /// The path to the fully resolved supergraph YAML.
    ///
    /// NOTE: Each subgraph entry MUST contain raw SDL
    /// as the schema source.
    config_file: Option<Utf8PathBuf>,
}

impl Compose {
    pub fn run(&self) -> ! {
        let composition_result = self.do_compose();

        print!("{}", serde_json::json!(composition_result));

        if composition_result.is_ok() {
            std::process::exit(0);
        } else {
            std::process::exit(1);
        }
    }

    fn do_compose(&self) -> BuildResult {
        let buffer = match &self.config_file {
            None => {
                let mut buffer = String::new();
                std::io::stdin()
                    .read_to_string(&mut buffer)
                    .map_err(|e| -> ConfigError {
                        ConfigError::InvalidConfiguration {
                            message: e.to_string(),
                        }
                    })?;
                Ok(buffer)
            }
            Some(config_path) => {
                if config_path.exists() {
                    let contents = std::fs::read_to_string(config_path).map_err(|e| {
                        ConfigError::MissingFile {
                            file_path: config_path.to_string(),
                            message: e.to_string(),
                        }
                    })?;
                    Ok(contents)
                } else {
                    Err(ConfigError::MissingFile {
                        file_path: config_path.to_string(),
                        message: "Unable to find supergraph.yaml file".to_owned(),
                    })
                }
            }
        }?;

        let supergraph_config = SupergraphConfig::new_from_yaml(&buffer)?;

        if let Some(federation_version) = supergraph_config.get_federation_version() {
            if !matches!(federation_version.get_major_version(), 2) {
                return Err(ConfigError::InvalidConfiguration {message: format!("Provided yaml resolved to 'federation_version: {}', which doesn't match the current supergraph binary.", federation_version )}.into());
            }
        }
        let subgraph_definitions = supergraph_config.get_subgraph_definitions()?;
        harmonize(subgraph_definitions)
    }
}
