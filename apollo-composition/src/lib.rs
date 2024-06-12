use apollo_compiler::Schema;
use apollo_federation::sources::connect::{validate, Location, ValidationErrorCode};

use apollo_federation_types::build::SubgraphDefinition;
use apollo_federation_types::build_plugin::{
    BuildMessage, BuildMessageLevel, BuildMessageLocation, BuildMessagePoint,
};

/// This trait includes all the Rust-side composition logic, plus hooks for the JavaScript side.
/// If you implement the functions in this trait to build your own JavaScript interface, then you
/// can call [`HybridComposition::compose`] to run the complete composition process.
///
/// JavaScript should be implemented using `@apollo/composition@2.9.0-connectors.0`.
#[allow(async_fn_in_trait)]
pub trait HybridComposition {
    /// Call the JavaScript `composeServices` function from `@apollo/composition` plus whatever
    /// extra logic you need. Make sure to disable satisfiability, like `composeServices(definitions, {}, false)`
    async fn compose_services_without_satisfiability(
        &mut self,
        subgraph_definitions: Vec<SubgraphDefinition>,
    ) -> Option<SupergraphSdl>;

    /// Call the JavaScript `validateSatisfiability` function from `@apollo/composition` plus whatever
    /// extra logic you need.
    async fn validate_satisfiability(&mut self, supergraph_sdl: String);

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
    /// 2. Call [`compose_services_without_satisfiability`] to run JavaScript-based composition
    /// 3. Run Rust-based validation on the supergraph
    /// 4. Call [`validate_satisfiability`] to run JavaScript-based validation on the supergraph
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
                    .map(|locations| SubgraphLocation {
                        subgraph: subgraph.name.clone(),
                        start: locations.start,
                        end: locations.end,
                    })
                    .collect(),
                severity: Severity::Error, // TODO: handle hints from apollo-federation
            })
        });
        self.add_issues(subgraph_validation_errors);

        let Some(supergraph_sdl) = self
            .compose_services_without_satisfiability(subgraph_definitions)
            .await
        else {
            return;
        };
        // TODO: transform supergraph_sdl to expand connectors
        let supergraph_sdl = String::from(supergraph_sdl);
        self.validate_satisfiability(supergraph_sdl).await;
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
    pub locations: Vec<SubgraphLocation>,
    pub severity: Severity,
}

/// A location in a subgraph's SDL
#[derive(Clone, Debug)]
pub struct SubgraphLocation {
    pub subgraph: String,
    pub start: Location,
    pub end: Location,
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

impl From<Severity> for BuildMessageLevel {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::Error => BuildMessageLevel::Error,
            Severity::Warning => BuildMessageLevel::Warn,
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

impl From<SubgraphLocation> for BuildMessageLocation {
    fn from(location: SubgraphLocation) -> Self {
        BuildMessageLocation {
            subgraph: Some(location.subgraph),
            start: Some(BuildMessagePoint {
                line: Some(location.start.line + 1),
                column: Some(location.start.column + 1),
                start: None,
                end: None,
            }),
            end: Some(BuildMessagePoint {
                line: Some(location.end.line + 1),
                column: Some(location.end.column + 1),
                start: None,
                end: None,
            }),
            source: None,
            other: Default::default(),
        }
    }
}
