mod config_error;
mod subgraph;
mod supergraph;

pub use config_error::ConfigError;
pub type ConfigResult<T> = std::result::Result<T, ConfigError>;
pub use subgraph::{SchemaSource, SubgraphConfig};
pub use supergraph::SupergraphConfig;
