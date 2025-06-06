//! Types used with the `apollo-composition` crate

use crate::build_plugin::{
    BuildMessage, BuildMessageLevel, BuildMessageLocation, BuildMessagePoint,
};
use crate::javascript::{CompositionHint, GraphQLError, SubgraphASTNode};
use crate::rover::{BuildError, BuildHint};
use apollo_compiler::parser::LineColumn;
use apollo_federation::error::FederationError;
use apollo_federation::subgraph::SubgraphError;
use std::collections::HashSet;
use std::ops::Range;

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

// thrown from expand_connectors and Supergraph::parse
impl From<FederationError> for Issue {
    fn from(error: FederationError) -> Self {
        let code = match &error {
            FederationError::SingleFederationError(err) => {
                err.code().definition().code().to_string()
            }
            _ => "UNKNOWN_ERROR_CODE".to_string(),
        };
        Issue {
            code,
            // Composition failed due to an internal error, please report this: {}
            message: error.to_string(),
            locations: vec![],
            severity: Severity::Error,
        }
    }
}

// impl From<CompositionError> for Issue {
//     fn from(error: CompositionError) -> Self {
//         Issue {
//             code: error.code().definition().code().to_string(),
//             // Composition failed due to an internal error, please report this: {}
//             message: error.to_string(),
//             // TODO CompositionError should specify locations
//             locations: vec![],
//             severity: Severity::Error,
//         }
//     }
// }

/// Rover and GraphOS expect messages to start with `[subgraph name]`. (They
/// don't actually look at the `locations` field, sadly). This will prepend
/// the subgraph name if there's exactly one. If there's more than one, it's
/// probably a composition issue that's not attributable to a single subgraph,
/// and GraphOS will show "[subgraph unknown]", which is also not correct.
fn maybe_prepend_subgraph(message: &str, locations: &[SubgraphLocation]) -> String {
    if message.starts_with('[') {
        return message.to_string();
    }
    let unique_subgraphs = locations
        .iter()
        .filter_map(|l| l.subgraph.as_ref())
        .collect::<HashSet<_>>();
    if unique_subgraphs.len() == 1 {
        format!(
            "[{}] {}",
            unique_subgraphs.iter().next().expect("qed"),
            message
        )
    } else {
        message.to_string()
    }
}

impl From<Issue> for BuildMessage {
    fn from(issue: Issue) -> Self {
        BuildMessage {
            level: issue.severity.into(),
            message: maybe_prepend_subgraph(&issue.message, &issue.locations),
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

pub fn convert_subraph_error_to_issues(error: SubgraphError) -> Vec<Issue> {
    error
        .format_errors()
        .into_iter()
        .map(|(code, message)| Issue {
            code,
            message,
            locations: vec![],
            severity: Severity::Error,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rstest::rstest]
    #[case("hello", &[], "hello")]
    #[case("hello", &[SubgraphLocation { subgraph: Some("subgraph".to_string()), range: None }], "[subgraph] hello")]
    #[case("[other] hello", &[SubgraphLocation { subgraph: Some("subgraph".to_string()), range: None }], "[other] hello")]
    #[case("hello", &[SubgraphLocation { subgraph: Some("subgraph".to_string()), range: None }, SubgraphLocation { subgraph: Some("other".to_string()), range: None }], "hello")]
    fn test_maybe_prepend_subgraph(
        #[case] message: &str,
        #[case] locations: &[SubgraphLocation],
        #[case] expected: &str,
    ) {
        assert_eq!(maybe_prepend_subgraph(message, locations), expected);
    }
}
