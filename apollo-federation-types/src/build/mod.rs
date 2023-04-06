mod error;
mod hint;
mod output;
mod subgraph_definition;

/// The type representing the result of a supergraph build (for any version)
pub type BuildResult = std::result::Result<BuildOutput, BuildErrors>;
pub use error::{BuildError, BuildErrorType, BuildErrors};
pub use hint::BuildHint;
pub use output::BuildOutput;
pub use subgraph_definition::SubgraphDefinition;
