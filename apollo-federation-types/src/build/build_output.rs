use serde::{Deserialize, Serialize};

/// BuildOutput contains information about the supergraph that was composed.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BuildOutput {
    /// Supergraph SDL can be used to start a gateway instance.
    pub supergraph_sdl: String,

    /// Hints contain information about the composition and should be displayed.
    pub hints: Vec<String>,

    /// Other untyped JSON included in the build output.
    #[serde(flatten)]
    other: crate::UncaughtJson,
}

impl BuildOutput {
    /// Create output containing only a supergraph schema
    pub fn new(supergraph_sdl: &str) -> Self {
        Self::new_with_hints(supergraph_sdl, &[])
    }

    /// Create output containing a supergraph schema and some hints
    pub fn new_with_hints(supergraph_sdl: &str, hints: &[&str]) -> Self {
        Self {
            supergraph_sdl: supergraph_sdl.to_string(),
            hints: hints.iter().map(|s| s.to_string()).collect(),
            other: crate::UncaughtJson::new(),
        }
    }
}
