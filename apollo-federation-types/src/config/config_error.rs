use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Could not parse supergraph config: {message}.")]
    InvalidConfiguration { message: String },

    #[error("File \"{file_path}\" not found: {message}.")]
    MissingFile { file_path: String, message: String },

    #[error("Config for subgraph(s) {subgraph_names} are not fully resolved. name, routing_url, and sdl must be present.")]
    SubgraphsNotResolved { subgraph_names: String },

    #[error("No subgraphs were found in the supergraph config.")]
    NoSubgraphsFound,
}

impl ConfigError {
    pub fn message(&self) -> String {
        self.to_string()
    }

    pub fn code(&self) -> Option<String> {
        // TODO: Eventually add codes to these?
        None
    }
}
