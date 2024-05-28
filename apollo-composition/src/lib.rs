use apollo_compiler::Schema;
use apollo_federation::sources::connect::{validate, ValidationErrorCode};

use apollo_federation_types::build::{
    BuildError, BuildErrorNode, BuildHint, BuildResult, SubgraphDefinition,
};

/// Runs the complete composition process, hooking into both the Rust and JavaScript implementations.
///
/// # Asyncness
///
/// While this function is async to allow for flexible JavaScript execution, it is a CPU-heavy task.
/// Take care when consuming this in an async context, as it may block longer than desired.
pub async fn compose<JavaScript: JavaScriptExecutor>(
    subgraph_definitions: Vec<SubgraphDefinition>,
) -> Result<PartialSuccess, Vec<Issue>> {
    let subgraph_validation_errors: Vec<Issue> = subgraph_definitions
        .iter()
        .flat_map(|subgraph| {
            // TODO: Use parse_and_validate (adding in directives as needed)
            // TODO: Handle schema errors rather than relying on JavaScript to catch it later
            let schema = Schema::parse(&subgraph.sdl, &subgraph.name)
                .unwrap_or_else(|schema_with_errors| schema_with_errors.partial);
            validate(schema).into_iter().map(|validation_error| Issue {
                code: transform_code(validation_error.code),
                message: validation_error.to_string(),
                locations: validation_error
                    .locations
                    .into_iter()
                    .map(|location| Location {
                        subgraph: subgraph.name.clone(),
                        start: LocationToken {
                            line: location.start_line - 1, // TODO: Return zero-indexed from apollo-federation
                            column: location.start_column - 1,
                        },
                        end: LocationToken {
                            line: location.end_line - 1,
                            column: location.end_column - 1,
                        },
                    })
                    .collect(),
                severity: Severity::Error, // TODO: handle hints from apollo-federation
            })
        })
        .collect();
    match JavaScript::compose(subgraph_definitions).await {
        Ok(result) => {
            if !subgraph_validation_errors.is_empty() {
                Err(subgraph_validation_errors)
            } else {
                // TODO: Run Rust-based supergraph validation after any JavaScript checks
                /* TODO: Do not duplicate Rust and JavaScript checks—either by removing pieces from
                  JS as they are implemented in Rust or by running more specific pieces of
                  JS code
                */
                Ok(PartialSuccess {
                    supergraph_sdl: result.supergraph_sdl,
                    issues: result.hints.into_iter().map(Issue::from).collect(),
                })
            }
        }
        Err(errors) => Err(subgraph_validation_errors
            .into_iter()
            .chain(errors.into_iter().map(Issue::from))
            .collect()),
    }
}

pub trait JavaScriptExecutor {
    #[allow(async_fn_in_trait)]
    async fn compose(subgraph_definitions: Vec<SubgraphDefinition>) -> BuildResult;
}

/// A successfully composed supergraph, optionally with some issues that should be addressed.
#[derive(Clone, Debug)]
pub struct PartialSuccess {
    pub supergraph_sdl: String,
    pub issues: Vec<Issue>,
}

/// Some issue the user should address. Errors block composition, warnings do not.
#[derive(Clone, Debug)]
pub struct Issue {
    pub code: String,
    pub message: String,
    pub locations: Vec<Location>,
    pub severity: Severity,
}

/// A location in a subgraph's SDL
#[derive(Clone, Debug)]
pub struct Location {
    pub subgraph: String,
    pub start: LocationToken,
    pub end: LocationToken,
}

/// zero-indexed line and column numbers
#[derive(Clone, Copy, Debug)]
pub struct LocationToken {
    pub line: usize,
    pub column: usize,
}

impl LocationToken {
    // A helper to return a location that is 0, 0 (for when a JavaScript error is missing location info)
    pub fn zeroed() -> Self {
        LocationToken { line: 0, column: 0 }
    }
}

impl From<BuildHint> for Issue {
    fn from(hint: BuildHint) -> Self {
        Issue {
            code: hint.code.unwrap_or_else(|| "UNKNOWN_HINT".to_string()),
            message: hint.message,
            locations: transform_nodes(hint.nodes.unwrap_or_default()),
            severity: Severity::Warning,
        }
    }
}

impl From<BuildError> for Issue {
    fn from(error: BuildError) -> Self {
        Issue {
            code: error.code.unwrap_or_else(|| "UNKNOWN_ERROR".to_string()),
            message: error.message.unwrap_or_default(),
            locations: transform_nodes(error.nodes.unwrap_or_default()),
            severity: Severity::Error,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

fn transform_nodes(locations: Vec<BuildErrorNode>) -> Vec<Location> {
    locations
        .into_iter()
        .map(|location| Location {
            subgraph: location.subgraph.unwrap_or_default(),
            start: location
                .start
                .map(|start| LocationToken {
                    line: start.line.unwrap_or_default(),
                    column: start.column.unwrap_or_default(),
                })
                .unwrap_or_else(LocationToken::zeroed),
            end: location
                .end
                .map(|end| LocationToken {
                    line: end.line.unwrap_or_default(),
                    column: end.column.unwrap_or_default(),
                })
                .unwrap_or_else(LocationToken::zeroed),
        })
        .collect()
}

fn transform_code(code: ValidationErrorCode) -> String {
    match code {
        ValidationErrorCode::GraphQLError => "GRAPHQL_ERROR".to_string(),
        ValidationErrorCode::DuplicateSourceName => "DUPLICATE_SOURCE_NAME".to_string(),
        ValidationErrorCode::InvalidSourceName => "INVALID_SOURCE_NAME".to_string(),
        ValidationErrorCode::EmptySourceName => "EMPTY_SOURCE_NAME".to_string(),
        ValidationErrorCode::SourceUrl => "SOURCE_URL".to_string(),
        ValidationErrorCode::SourceScheme => "SOURCE_SCHEME".to_string(),
    }
}
