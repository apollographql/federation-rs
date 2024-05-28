use apollo_composition::{compose, Location};
use camino::Utf8PathBuf;
use structopt::StructOpt;

use apollo_federation_types::build::{
    BuildError, BuildErrorNode, BuildErrorNodeLocationToken, BuildErrors, BuildHint, BuildOutput,
};
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
        compose::<Harmonizer>(subgraph_definitions)
            .await
            .map(|success| BuildOutput {
                supergraph_sdl: success.supergraph_sdl,
                hints: success
                    .issues
                    .into_iter()
                    .map(|issue| BuildHint {
                        message: issue.message,
                        code: Some(issue.code),
                        nodes: Some(transform_locations(issue.locations)),
                        omitted_nodes_count: None,
                        other: Default::default(),
                    })
                    .collect(),
                other: Default::default(),
            })
            .map_err(|errors| {
                BuildErrors::from_iter(errors.into_iter().map(|error| {
                    BuildError::composition_error(
                        Some(error.code),
                        Some(error.message),
                        Some(transform_locations(error.locations)),
                        None,
                    )
                }))
            })
    }
}

fn transform_locations(locations: Vec<Location>) -> Vec<BuildErrorNode> {
    locations
        .into_iter()
        .map(|location| BuildErrorNode {
            subgraph: Some(location.subgraph),
            source: None,
            start: Some(BuildErrorNodeLocationToken {
                line: Some(location.start.line + 1),
                column: Some(location.start.column + 1),
                start: None,
                end: None,
            }),
            end: Some(BuildErrorNodeLocationToken {
                line: Some(location.end.line + 1),
                column: Some(location.end.column + 1),
                start: None,
                end: None,
            }),
        })
        .collect()
}

#[tokio::test]
async fn compose_test() {
    let res = Compose {
        config_file: "./tests/compose_test.yaml".into(),
    };

    let result = res.do_compose().await;

    insta::assert_json_snapshot!(result);
}
