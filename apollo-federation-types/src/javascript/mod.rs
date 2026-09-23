use apollo_federation::subgraph::typestate::{Initial, Subgraph, Validated};
use apollo_federation::subgraph::SubgraphError;
use serde::{Deserialize, Serialize};

/// The `SubgraphDefinition` represents everything we need to know about a
/// subgraph for its GraphQL runtime responsibilities.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct SubgraphDefinition {
    /// The name of the subgraph. We use this name internally to
    /// in the representation of the composed schema and for designations
    /// within the human-readable QueryPlan.
    pub name: String,

    /// The routing/runtime URL where the subgraph can be found that will
    /// be able to fulfill the requests it is responsible for.
    pub url: String,

    /// The Schema Definition Language (SDL) containing the type definitions
    /// for a subgraph.
    pub sdl: String,
}

impl TryFrom<SubgraphDefinition> for Subgraph<Initial> {
    type Error = SubgraphError;

    fn try_from(value: SubgraphDefinition) -> Result<Self, Self::Error> {
        Subgraph::parse(value.name.as_str(), value.url.as_str(), value.sdl.as_str())
    }
}

impl From<Subgraph<Validated>> for SubgraphDefinition {
    fn from(value: Subgraph<Validated>) -> Self {
        SubgraphDefinition {
            sdl: value.schema_string(),
            name: value.name,
            url: value.url,
        }
    }
}
