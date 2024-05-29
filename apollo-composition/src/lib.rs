use apollo_compiler::Schema;
use apollo_federation::sources::connect::{validate, ValidationErrorCode};

use apollo_federation_types::build::SubgraphDefinition;

#[allow(async_fn_in_trait)]
pub trait Composer {
    /// Call the JavaScript `composeServices` function from `@apollo/composition` plus whatever
    /// extra logic you need.
    async fn compose_services(
        &mut self,
        subgraph_definitions: Vec<SubgraphDefinition>,
    ) -> Option<SupergraphSdl>;

    /// When the Rust composition/validation code finds issues, it will call this method to add
    /// them to the list of issues that will be returned to the user.
    ///
    /// It's on the implementor of this trait to convert `From<Issue>`
    fn add_issues<Source: Iterator<Item = Issue>>(&mut self, issues: Source);

    /// Runs the complete composition process, hooking into both the Rust and JavaScript implementations.
    ///
    /// # Asyncness
    ///
    /// While this function is async to allow for flexible JavaScript execution, it is a CPU-heavy task.
    /// Take care when consuming this in an async context, as it may block longer than desired.
    ///
    /// # Algorithm
    ///
    /// 1. Run Rust-based validation on the subgraphs
    /// 2. Call [`compose_services`] to run JavaScript-based composition
    /// 3. Run Rust-based validation on the supergraph
    async fn compose(&mut self, subgraph_definitions: Vec<SubgraphDefinition>) {
        let subgraph_validation_errors = subgraph_definitions.iter().flat_map(|subgraph| {
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
        });
        self.add_issues(subgraph_validation_errors);

        let Some(_supergraph_sdl) = self.compose_services(subgraph_definitions).await else {
            return; // JavaScript composition failed, we can't run any Rust validations.
        };
        // TODO: Run Rust-based supergraph validation after any JavaScript checks
        /* TODO: Do not duplicate Rust and JavaScript checks—either by removing pieces from
          JS as they are implemented in Rust or by running more specific pieces of
          JS code
        */
    }
}

pub type SupergraphSdl<'a> = &'a str;

/// A successfully composed supergraph, optionally with some issues that should be addressed.
#[derive(Clone, Debug)]
pub struct PartialSuccess {
    pub supergraph_sdl: String,
    pub issues: Vec<Issue>,
}

/// Some issue the user should address. Errors block composition, warnings do not.
#[derive(Clone, Debug)]
pub struct Issue {
    pub code: &'static str,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

fn transform_code(code: ValidationErrorCode) -> &'static str {
    match code {
        ValidationErrorCode::GraphQLError => "GRAPHQL_ERROR",
        ValidationErrorCode::DuplicateSourceName => "DUPLICATE_SOURCE_NAME",
        ValidationErrorCode::InvalidSourceName => "INVALID_SOURCE_NAME",
        ValidationErrorCode::EmptySourceName => "EMPTY_SOURCE_NAME",
        ValidationErrorCode::SourceUrl => "SOURCE_URL",
        ValidationErrorCode::SourceScheme => "SOURCE_SCHEME",
        ValidationErrorCode::SourceNameMismatch => "SOURCE_NAME_MISMATCH",
        ValidationErrorCode::SubscriptionInConnectors => "SUBSCRIPTION_IN_CONNECTORS",
    }
}
