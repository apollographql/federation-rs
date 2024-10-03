//! This module contains types matching those in the JavaScript `@apollo/composition` package.

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
