use apollo_compiler::parser::LineColumn;
use apollo_compiler::schema::ExtendedType;
use apollo_federation::sources::connect::expand::{expand_connectors, Connectors, ExpansionResult};
use apollo_federation::sources::connect::validation::{
    validate, Severity as ValidationSeverity, ValidationResult,
};
use apollo_federation_types::build_plugin::{
    BuildMessage, BuildMessageLevel, BuildMessageLocation, BuildMessagePoint,
};
use apollo_federation_types::javascript::{
    CompositionHint, GraphQLError, SatisfiabilityResult, SubgraphASTNode, SubgraphDefinition,
};
use apollo_federation_types::rover::{BuildError, BuildHint};
use either::Either;
use std::collections::HashMap;
use std::iter::once;
use std::ops::Range;

/// This trait includes all the Rust-side composition logic, plus hooks for the JavaScript side.
/// If you implement the functions in this trait to build your own JavaScript interface, then you
/// can call [`HybridComposition::compose`] to run the complete composition process.
///
/// JavaScript should be implemented using `@apollo/composition@2.9.0-connectors.0`.
#[allow(async_fn_in_trait)]
pub trait HybridComposition {
    /// Call the JavaScript `composeServices` function from `@apollo/composition` plus whatever
    /// extra logic you need. Make sure to disable satisfiability, like `composeServices(definitions, {runSatisfiability: false})`
    async fn compose_services_without_satisfiability(
        &mut self,
        subgraph_definitions: Vec<SubgraphDefinition>,
    ) -> Option<SupergraphSdl>;

    /// Call the JavaScript `validateSatisfiability` function from `@apollo/composition` plus whatever
    /// extra logic you need.
    ///
    /// # Input
    ///
    /// The `validateSatisfiability` function wants an argument like `{ supergraphSdl }`. That field
    /// should be the value that's updated when [`update_supergraph_sdl`] is called.
    ///
    /// # Output
    ///
    /// If satisfiability completes from JavaScript, the [`SatisfiabilityResult`] (matching the shape
    /// of that function) should be returned. If Satisfiability _can't_ be run, you can return an
    /// `Err(Issue)` instead indicating what went wrong.
    async fn validate_satisfiability(&mut self) -> Result<SatisfiabilityResult, Issue>;

    /// Allows the Rust composition code to modify the stored supergraph SDL
    /// (for example, to expand connectors).
    fn update_supergraph_sdl(&mut self, supergraph_sdl: String);

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
        let validation_results = subgraph_definitions
            .iter()
            .map(|subgraph| {
                (
                    subgraph.name.clone(),
                    validate(&subgraph.sdl, &subgraph.name),
                )
            })
            .collect::<Vec<_>>();
        let subgraph_validation_errors = validation_results
            .iter()
            .flat_map(|(name, validation_result)| {
                validation_result
                    .errors
                    .iter()
                    .cloned()
                    .map(|validation_error| Issue {
                        code: validation_error.code.to_string(),
                        message: validation_error.message,
                        locations: validation_error
                            .locations
                            .into_iter()
                            .map(|range| SubgraphLocation {
                                subgraph: Some(name.clone()),
                                range: Some(range),
                            })
                            .collect(),
                        severity: validation_error.code.severity().into(),
                    })
            })
            .collect::<Vec<_>>();

        let run_composition = subgraph_validation_errors
            .iter()
            .all(|issue| issue.severity != Severity::Error);
        self.add_issues(subgraph_validation_errors.into_iter());
        if !run_composition {
            return;
        }

        let Some(supergraph_sdl) = self
            .compose_services_without_satisfiability(subgraph_definitions)
            .await
        else {
            return;
        };

        // Any issues with overrides are fatal since they'll cause errors in expansion,
        // so we return early if we see any.
        let override_errors = validate_overrides(validation_results);
        if !override_errors.is_empty() {
            self.add_issues(override_errors.into_iter());
            return;
        }

        let expansion_result = match expand_connectors(supergraph_sdl) {
            Ok(result) => result,
            Err(err) => {
                self.add_issues(once(Issue {
                    code: "INTERNAL_ERROR".to_string(),
                    message: format!(
                        "Composition failed due to an internal error, please report this: {}",
                        err
                    ),
                    locations: vec![],
                    severity: Severity::Error,
                }));
                return;
            }
        };
        match expansion_result {
            ExpansionResult::Expanded {
                raw_sdl,
                connectors: Connectors {
                    by_service_name, ..
                },
                ..
            } => {
                let original_supergraph_sdl = supergraph_sdl.to_string();
                self.update_supergraph_sdl(raw_sdl);
                let satisfiability_result = self.validate_satisfiability().await;
                self.add_issues(
                    satisfiability_result_into_issues(satisfiability_result).map(|mut issue| {
                        for (service_name, connector) in by_service_name.iter() {
                            issue.message = issue
                                .message
                                .replace(&**service_name, connector.id.subgraph_name.as_str());
                        }
                        issue
                    }),
                );

                self.update_supergraph_sdl(original_supergraph_sdl);
            }
            ExpansionResult::Unchanged => {
                let satisfiability_result = self.validate_satisfiability().await;
                self.add_issues(satisfiability_result_into_issues(satisfiability_result));
            }
        }
    }
}

/// Validate overrides for connector-related subgraphs
///
/// Overrides mess with the supergraph in ways that can be difficult to detect when
/// expanding connectors; the supergraph may omit overridden fields and other shenanigans.
/// To allow for a better developer experience, we check here if any connector-enabled subgraphs
/// have fields overridden.
fn validate_overrides(schemas: impl IntoIterator<Item = (String, ValidationResult)>) -> Vec<Issue> {
    let validations_by_subgraph_name = HashMap::<_, _>::from_iter(schemas.into_iter());
    let mut override_errors = Vec::new();
    for (subgraph_name, ValidationResult { schema, .. }) in validations_by_subgraph_name.iter() {
        // We need to grab all fields in the schema since only fields can have the @override
        // directive attached
        macro_rules! extract_directives {
            ($node:ident) => {
                $node
                    .fields
                    .iter()
                    .flat_map(|(name, field)| {
                        field
                            .directives
                            .iter()
                            .map(move |d| (format!("{}.{}", $node.name, name), d))
                    })
                    .collect::<Vec<_>>()
            };
        }
        let override_directives = schema
            .types
            .values()
            .flat_map(|v| match v {
                ExtendedType::Object(node) => extract_directives!(node),
                ExtendedType::Interface(node) => extract_directives!(node),
                ExtendedType::InputObject(node) => extract_directives!(node),

                // These types do not have fields
                ExtendedType::Scalar(_) | ExtendedType::Union(_) | ExtendedType::Enum(_) => {
                    Vec::new()
                }
            })
            .filter(|(_, directive)| directive.name == "override");

        // Now see if we have any overrides that try to reference connector subgraphs
        for (field, directive) in override_directives {
            // If the override directive does not have a valid `from` field, then there is
            // no point trying to validate it, as later steps will validate the entire schema.
            let Ok(Some(overriden_subgraph_name)) = directive
                .argument_by_name("from", schema)
                .map(|node| node.as_str())
            else {
                continue;
            };

            if let Some(overriden_subgraph) =
                validations_by_subgraph_name.get(overriden_subgraph_name)
            {
                if overriden_subgraph.has_connectors {
                    override_errors.push(Issue {
                        code: "OVERRIDE_ON_CONNECTOR".to_string(),
                        message: format!(
                            r#"Field "{}" on subgraph "{}" is trying to override connector-enabled subgraph "{}", which is not yet supported. See https://go.apollo.dev/connectors/limitations#override-is-partially-unsupported"#,
                            field,
                            subgraph_name,
                            overriden_subgraph_name,
                        ),
                        locations: vec![SubgraphLocation {
                            subgraph: Some(subgraph_name.clone()),
                            range: directive.line_column_range(&schema.sources),
                        }],
                        severity: Severity::Error,
                    });
                }
            }
        }
    }

    override_errors
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
    pub code: String,
    pub message: String,
    pub locations: Vec<SubgraphLocation>,
    pub severity: Severity,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

impl From<ValidationSeverity> for Severity {
    fn from(severity: ValidationSeverity) -> Self {
        match severity {
            ValidationSeverity::Error => Severity::Error,
            ValidationSeverity::Warning => Severity::Warning,
        }
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

fn satisfiability_result_into_issues(
    satisfiability_result: Result<SatisfiabilityResult, Issue>,
) -> Either<impl Iterator<Item = Issue>, impl Iterator<Item = Issue>> {
    match satisfiability_result {
        Ok(satisfiability_result) => Either::Left(
            satisfiability_result
                .errors
                .into_iter()
                .flatten()
                .map(Issue::from)
                .chain(
                    satisfiability_result
                        .hints
                        .into_iter()
                        .flatten()
                        .map(Issue::from),
                ),
        ),
        Err(issue) => Either::Right(once(issue)),
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
