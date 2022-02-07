mod build_error;
mod build_output;
mod subgraph_definition;

/// The type representing the result of a supergraph build (for any version)
pub type BuildResult = std::result::Result<BuildOutput, BuildErrors>;
pub use build_error::{BuildError, BuildErrors};
pub use build_output::BuildOutput;
pub use subgraph_definition::SubgraphDefinition;
