use camino::Utf8PathBuf;
use structopt::StructOpt;

use apollo_federation_types::{
    build::BuildResult,
    config::{ConfigError, PluginVersion, SupergraphConfig},
};
use harmonizer::harmonize;

use std::fs::File;
use std::io::BufWriter;

#[derive(Debug, StructOpt)]
pub struct Compose {
    /// The path to the fully resolved supergraph YAML.
    ///
    /// NOTE: Each subgraph entry MUST contain raw SDL
    /// as the schema source.
    config_file: Utf8PathBuf,
    /// Output to a file. Default is to use stdout.
    output: Option<Utf8PathBuf>,
}

impl Compose {
    pub fn run(&self) -> ! {
        let composition_result = self.do_compose();
        let composition_succeeded = composition_result.is_ok();

        if let Some(filepath) = &self.output {
            let json = composition_result;
            let file =
                File::create(filepath).expect(&format!("failed to create file at ${filepath}"));
            let writer = BufWriter::new(file);
            serde_json::to_writer(writer, &json)
                .expect("failed to write composition result to file");
        } else {
            print!("{}", serde_json::json!(composition_result));
        }

        if composition_succeeded {
            std::process::exit(0);
        } else {
            std::process::exit(1);
        }
    }

    fn do_compose(&self) -> BuildResult {
        let supergraph_config = SupergraphConfig::new_from_yaml_file(&self.config_file)?;
        if let Some(federation_version) = supergraph_config.get_federation_version() {
            if !matches!(federation_version.get_major_version(), 2) {
                return Err(ConfigError::InvalidConfiguration {message: format!("It looks like '{}' resolved to 'federation_version: {}', which doesn't match the current supergraph binary.", &self.config_file, federation_version )}.into());
            }
        }
        let subgraph_definitions = supergraph_config.get_subgraph_definitions()?;
        harmonize(subgraph_definitions)
    }
}

#[test]
fn compose_test() {
    let res = Compose {
        config_file: "./tests/compose_test.yaml".into(),
        output: None,
    };

    let result = res.do_compose();

    insta::assert_json_snapshot!(result);
}
