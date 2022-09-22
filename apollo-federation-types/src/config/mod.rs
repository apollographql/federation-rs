mod config_error;
mod subgraph;
mod supergraph;
mod version;

pub use config_error::ConfigError;
pub use version::{FederationVersion, PluginVersion, RouterVersion};
pub type ConfigResult<T> = std::result::Result<T, ConfigError>;
pub use subgraph::{SchemaSource, SubgraphConfig};
pub use supergraph::SupergraphConfig;
