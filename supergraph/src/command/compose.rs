use apollo_composition::Composer;
use camino::Utf8PathBuf;
use structopt::StructOpt;

use apollo_federation_types::build::BuildOutput;
use apollo_federation_types::{
    build::BuildResult,
    config::{ConfigError, PluginVersion, SupergraphConfig},
};
use harmonizer::Harmonizer;

#[derive(Debug, StructOpt)]
pub struct Compose {
    /// The path to the fully resolved supergraph YAML.
    ///
    /// NOTE: Each subgraph entry MUST contain raw SDL
    /// as the schema source.
    config_file: Utf8PathBuf,
}

impl Compose {
    pub async fn run(&self) -> ! {
        let composition_result = self.do_compose().await;

        print!("{}", serde_json::json!(composition_result));

        if composition_result.is_ok() {
            std::process::exit(0);
        } else {
            std::process::exit(1);
        }
    }

    async fn do_compose(&self) -> BuildResult {
        let supergraph_config = SupergraphConfig::new_from_yaml_file(&self.config_file)?;
        if let Some(federation_version) = supergraph_config.get_federation_version() {
            if !matches!(federation_version.get_major_version(), 2) {
                return Err(ConfigError::InvalidConfiguration {message: format!("It looks like '{}' resolved to 'federation_version: {}', which doesn't match the current supergraph binary.", &self.config_file, federation_version )}.into());
            }
        }
        let subgraph_definitions = supergraph_config.get_subgraph_definitions()?;
        let mut harmonizer = Harmonizer::default();
        harmonizer.compose(subgraph_definitions).await;

        match harmonizer.supergraph_sdl {
            Some(supergraph_sdl) if harmonizer.errors.is_empty() => Ok(
                BuildOutput::new_with_hints(supergraph_sdl, harmonizer.hints),
            ),
            _ => Err(harmonizer.errors),
        }
    }
}

#[tokio::test]
async fn compose_test() {
    let res = Compose {
        config_file: "./tests/compose_test.yaml".into(),
    };

    let result = res.do_compose().await;

    insta::assert_json_snapshot!(result);
}
