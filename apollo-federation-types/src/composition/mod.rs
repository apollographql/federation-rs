//! Types used with the `apollo-composition` crate

use std::ops::Range;

use apollo_compiler::parser::LineColumn;

use crate::build_plugin::{
    BuildMessage, BuildMessageLevel, BuildMessageLocation, BuildMessagePoint,
};
use crate::javascript::{CompositionHint, GraphQLError, SubgraphASTNode};
use crate::rover::{BuildError, BuildHint};

/// Some issue the user should address. Errors block composition, warnings do not.
#[derive(Clone, Debug)]
pub struct Issue {
    pub code: String,
    pub message: String,
    pub locations: Vec<SubgraphLocation>,
    pub severity: Severity,
}

impl From<GraphQLError> for Issue {
    fn from(error: GraphQLError) -> Issue {
        Issue {
            code: error
                .extensions
                .map(|extension| extension.code)
                .unwrap_or_default(),
            message: error.message,
            severity: Severity::Error,
            locations: error
                .nodes
                .unwrap_or_default()
                .into_iter()
                .filter_map(SubgraphLocation::from_ast)
                .collect(),
        }
    }
}

impl From<CompositionHint> for Issue {
    fn from(hint: CompositionHint) -> Issue {
        Issue {
            code: hint.definition.code,
            message: hint.message,
            severity: Severity::Warning,
            locations: hint
                .nodes
                .unwrap_or_default()
                .into_iter()
                .filter_map(SubgraphLocation::from_ast)
                .collect(),
        }
    }
}

impl From<BuildError> for Issue {
    fn from(error: BuildError) -> Issue {
        Issue {
            code: error
                .code
                .unwrap_or_else(|| "UNKNOWN_ERROR_CODE".to_string()),
            message: error.message.unwrap_or_else(|| "Unknown error".to_string()),
            locations: error
                .nodes
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            severity: Severity::Error,
        }
    }
}

impl From<BuildHint> for Issue {
    fn from(hint: BuildHint) -> Issue {
        Issue {
            code: hint.code.unwrap_or_else(|| "UNKNOWN_HINT_CODE".to_string()),
            message: hint.message,
            locations: hint
                .nodes
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            severity: Severity::Warning,
        }
    }
}

impl From<Issue> for BuildMessage {
    fn from(issue: Issue) -> Self {
        BuildMessage {
            level: issue.severity.into(),
            message: issue.message,
            code: Some(issue.code.to_string()),
            locations: issue
                .locations
                .into_iter()
                .map(|location| location.into())
                .collect(),
            schema_coordinate: None,
            step: None,
            other: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

impl From<Severity> for BuildMessageLevel {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::Error => BuildMessageLevel::Error,
            Severity::Warning => BuildMessageLevel::Warn,
        }
    }
}

/// A location in a subgraph's SDL
#[derive(Clone, Debug)]
pub struct SubgraphLocation {
    /// This field is an Option to support the lack of subgraph names in
    /// existing composition errors. New composition errors should always
    /// include a subgraph name.
    pub subgraph: Option<String>,
    pub range: Option<Range<LineColumn>>,
}

impl SubgraphLocation {
    fn from_ast(node: SubgraphASTNode) -> Option<Self> {
        Some(Self {
            subgraph: node.subgraph,
            range: node.loc.and_then(|node_loc| {
                Some(Range {
                    start: LineColumn {
                        line: node_loc.start_token.line?,
                        column: node_loc.start_token.column?,
                    },
                    end: LineColumn {
                        line: node_loc.end_token.line?,
                        column: node_loc.end_token.column?,
                    },
                })
            }),
        })
    }
}

impl From<SubgraphLocation> for BuildMessageLocation {
    fn from(location: SubgraphLocation) -> Self {
        BuildMessageLocation {
            subgraph: location.subgraph,
            start: location.range.as_ref().map(|range| BuildMessagePoint {
                line: Some(range.start.line),
                column: Some(range.start.column),
                start: None,
                end: None,
            }),
            end: location.range.as_ref().map(|range| BuildMessagePoint {
                line: Some(range.end.line),
                column: Some(range.end.column),
                start: None,
                end: None,
            }),
            source: None,
            other: Default::default(),
        }
    }
}

impl From<BuildMessageLocation> for SubgraphLocation {
    fn from(location: BuildMessageLocation) -> Self {
        Self {
            subgraph: location.subgraph,
            range: location.start.and_then(|start| {
                let end = location.end?;
                Some(Range {
                    start: LineColumn {
                        line: start.line?,
                        column: start.column?,
                    },
                    end: LineColumn {
                        line: end.line?,
                        column: end.column?,
                    },
                })
            }),
        }
    }
}
