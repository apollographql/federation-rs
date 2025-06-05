//! This module contains types matching those in the JavaScript `@apollo/composition` package.

use apollo_federation::subgraph::typestate::{Initial, Subgraph, Upgraded, Validated};
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

/// The structure returned by `validateSatisfiability` in `@apollo/composition`
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct SatisfiabilityResult {
    pub errors: Option<Vec<GraphQLError>>,
    pub hints: Option<Vec<CompositionHint>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct MergeResult {
    pub supergraph: String,
    pub hints: Option<Vec<CompositionHint>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct CompositionHint {
    pub message: String,
    pub nodes: Option<Vec<SubgraphASTNode>>,
    pub definition: HintCodeDefinition,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct HintCodeDefinition {
    pub code: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct SubgraphASTNode {
    pub loc: Option<Location>,
    pub subgraph: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub start_token: Token,
    pub end_token: Token,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct Token {
    pub column: Option<usize>,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct GraphQLError {
    pub message: String,
    pub nodes: Option<Vec<SubgraphASTNode>>,
    pub extensions: Option<GraphQLErrorExtensions>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct GraphQLErrorExtensions {
    pub code: String,
}

impl TryFrom<SubgraphDefinition> for Subgraph<Initial> {
    type Error = SubgraphError;

    fn try_from(value: SubgraphDefinition) -> Result<Self, Self::Error> {
        Subgraph::parse(value.name.as_str(), value.url.as_str(), value.sdl.as_str())
    }
}

impl From<Subgraph<Upgraded>> for SubgraphDefinition {
    fn from(value: Subgraph<Upgraded>) -> Self {
        SubgraphDefinition {
            sdl: value.schema_string(),
            name: value.name,
            url: value.url,
        }
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

// converts subgraph definitions to Subgraph<Upgraded> by assuming schema is valid and was already upgraded
pub fn assume_subgraph_upgraded(
    definition: SubgraphDefinition,
) -> Result<Subgraph<Upgraded>, SubgraphError> {
    Subgraph::parse(
        definition.name.as_str(),
        definition.url.as_str(),
        definition.sdl.as_str(),
    )
    .and_then(|s| s.assume_expanded())
    .map(|s| s.assume_upgraded())
}

// converts subgraph definitions to Subgraph<Validated> by assuming schema is valid and was already validated
pub fn assume_subgraph_validated(
    definition: SubgraphDefinition,
) -> Result<Subgraph<Validated>, SubgraphError> {
    assume_subgraph_upgraded(definition).and_then(|s| s.assume_validated())
}
