mod error;
mod hint;
mod output;
mod subgraph_definition;

/// The type representing the result of a supergraph build (for any version)
pub type BuildResult = Result<BuildOutput, BuildErrors>;

impl From<PluginResult> for BuildResult {
    fn from(value: PluginResult) -> Self {
        let mut hints = Vec::new();
        let mut errors = Vec::new();
        for message in value.build_messages {
            match message.level {
                BuildMessageLevel::Error => {
                    errors.push(BuildError::from(message));
                }
                _ => {
                    hints.push(BuildHint::from(message));
                }
            }
        }
        value
            .result
            .map(|supergraph_sdl| BuildOutput {
                supergraph_sdl,
                hints,
                other: Default::default(),
            })
            .map_err(|reason| BuildErrors {
                build_errors: errors,
                is_config: reason == PluginFailureReason::Config,
            })
    }
}

use crate::build_plugin::{BuildMessageLevel, PluginFailureReason, PluginResult};
pub use error::{BuildError, BuildErrorType, BuildErrors};
pub use hint::BuildHint;
pub use output::BuildOutput;
pub use subgraph_definition::SubgraphDefinition;
