use either::Either;
use std::collections::HashMap;
use std::iter::once;

use apollo_compiler::collections::IndexMap;
use apollo_compiler::{schema::ExtendedType, Schema};
use apollo_federation::composition::{
    expand_subgraphs, merge_subgraphs, post_merge_validations, pre_merge_validations,
    upgrade_subgraphs_if_necessary, validate_satisfiability, validate_subgraphs, Supergraph,
};
use apollo_federation::sources::connect::{
    expand::{expand_connectors, Connectors, ExpansionResult},
    validation::{validate, Severity as ValidationSeverity, ValidationResult},
};
use apollo_federation::subgraph::typestate::{Initial, Subgraph, Upgraded, Validated};
use apollo_federation::subgraph::SubgraphError;
use apollo_federation_types::composition::SubgraphLocation;
use apollo_federation_types::javascript::{CompositionHint, CompositionResult, HintCodeDefinition};
use apollo_federation_types::{
    composition::{Issue, Severity},
    javascript::{SatisfiabilityResult, SubgraphDefinition},
};

type ValidatedConnectorSubgraphs = IndexMap<String, ValidationResult>;

type ValidatedConnectorSubgraphs = IndexMap<String, ValidationResult>;

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
        let mut subgraph_validation_errors = Vec::new();
        let mut parsed_schemas = HashMap::new();
        let subgraph_definitions = subgraph_definitions
            .into_iter()
            .map(|mut subgraph| {
                let ValidationResult {
                    errors,
                    has_connectors,
                    schema,
                    transformed,
                } = validate(subgraph.sdl, &subgraph.name);
                subgraph.sdl = transformed;
                for error in errors {
                    subgraph_validation_errors.push(Issue {
                        code: error.code.to_string(),
                        message: error.message,
                        locations: error
                            .locations
                            .into_iter()
                            .map(|range| SubgraphLocation {
                                subgraph: Some(subgraph.name.clone()),
                                range: Some(range),
                            })
                            .collect(),
                        severity: convert_severity(error.code.severity()),
                    })
                }
                parsed_schemas.insert(
                    subgraph.name.clone(),
                    SubgraphSchema {
                        schema,
                        has_connectors,
                    },
                );
                subgraph
            })
            .collect();

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
        let override_errors = validate_overrides(parsed_schemas);
        if !override_errors.is_empty() {
            self.add_issues(override_errors.into_iter());
            return;
        }

        let expansion_result = match expand_connectors(supergraph_sdl, &Default::default()) {
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

    /// <div class="warning">*** EXPERIMENTAL ***</div>
    ///
    /// Runs the composition process with granular composition phases that allow replacing individual
    /// steps with Rust and/or JavaScript implementations.
    ///
    /// 1. <connectors> subgraph validation
    /// 2. Initialize subgraphs - parses SDL into a GraphQL schema
    /// 3. Expands subgraphs - adds all missing federation definitions
    /// 4. Upgrade subgraphs - upgrades fed v1 schemas to fed v2
    /// 5. Validate subgraphs
    /// 6. Pre-merge validations (includes connectors validations)
    /// 7. Merge subgraphs into a supergrpah
    /// 8. Post merge validations
    /// 9. <connectors> expand supergraph
    /// 10. Validate satisfiability
    ///
    /// In case of a composition failure, we return a list of errors from the current composition
    /// phase.
    async fn experimental_compose(
        &mut self,
        subgraph_definitions: Vec<SubgraphDefinition>,
    ) -> Result<CompositionResult, Vec<Issue>> {
        // connectors subgraph validations
        let (modified_subgraphs, parsed_subgraphs) =
            match self.validate_connector_subgraphs(subgraph_definitions) {
                Ok(results) => results,
                Err(errors) => {
                    return Err(errors);
                }
            };
        let upgraded_subgraphs = self
            .experimental_upgrade_subgraphs(subgraph_definitions)
            .await?;
        let validated_subgraphs = self
            .experimental_validate_subgraphs(upgraded_subgraphs)
            .await?;

        // connectors supergraph pre merge validations
        // Any issues with overrides are fatal since they'll cause errors in expansion,
        // so we return early if we see any.
        // TODO this should be run as last step of validate_connector_subgraphs
        let override_errors = validate_overrides(parsed_subgraphs);
        if !override_errors.is_empty() {
            return Err(override_errors);
        }
        let merge_result = self
            .experimental_merge_subgraphs(validated_subgraphs)
            .await?;

        // expand connectors as needed
        let supergraph_sdl = merge_result.supergraph.clone();
        let expansion_result = match expand_connectors(&supergraph_sdl, &Default::default()) {
            Ok(result) => result,
            Err(err) => {
                return Err(vec![err.into()]);
            }
        };
        match expansion_result {
            ExpansionResult::Expanded {
                raw_sdl,
                // connectors: Connectors {
                //     by_service_name, ..
                // },
                ..
            } => {
                self.experimental_validate_satisfiability(raw_sdl.as_str())
                    .await
                    .map(|r| {
                        let final_hints = merge_result
                            .hints
                            .into_iter()
                            .chain(r.hints.into_iter())
                            .reduce(|mut h1, mut h2| {
                                h1.append(&mut h2);
                                h1
                            });
                        // return original supergraph
                        CompositionResult {
                            supergraph: supergraph_sdl,
                            hints: final_hints,
                        }
                    })
                // TODO update connector names in the satisfiability errors
                // .map_err(|err| {
                //     err.into_iter()
                //         .map(|e| {
                //             for (service_name, connector) in by_service_name.iter() {
                //                 e.message = e.message
                //                     .replace(&**service_name, connector.id.subgraph_name.as_str());
                //             }
                //         })
                //         .collect()
                // })
            }
            ExpansionResult::Unchanged => {
                self.experimental_validate_satisfiability(supergraph_sdl.as_str())
                    .await
                    .map(|s| {
                        let final_hints = merge_result
                            .hints
                            .into_iter()
                            .chain(s.hints.into_iter())
                            .reduce(|mut h1, mut h2| {
                                h1.append(&mut h2);
                                h1
                            });
                        // return original supergraph
                        CompositionResult {
                            supergraph: supergraph_sdl,
                            hints: final_hints,
                        }
                    })
            }
        }
    }

    // TODO this should eventually move under expand/validate subgraph logic
    fn validate_connector_subgraphs(
        &mut self,
        subgraph_definitions: Vec<SubgraphDefinition>,
    ) -> Result<(Vec<SubgraphDefinition>, HashMap<String, SubgraphSchema>), Vec<Issue>> {
        let mut subgraph_validation_errors = Vec::new();
        let mut parsed_schemas = HashMap::new();
        let subgraph_definitions = subgraph_definitions
            .into_iter()
            .map(|mut subgraph| {
                let ValidationResult {
                    errors,
                    has_connectors,
                    schema,
                    transformed,
                } = validate(subgraph.sdl, &subgraph.name);
                subgraph.sdl = transformed;
                subgraph_validation_errors.extend(errors.into_iter()
                    .map(|error| Issue {
                        code: error.code.to_string(),
                        message: error.message,
                        locations: error
                            .locations
                            .into_iter()
                            .map(|range| SubgraphLocation {
                                subgraph: Some(subgraph.name.clone()),
                                range: Some(range),
                            })
                            .collect(),
                        severity: convert_severity(error.code.severity()),
                    }));
                parsed_schemas.insert(
                    subgraph.name.clone(),
                    SubgraphSchema {
                        schema,
                        has_connectors,
                    },
                );
                subgraph
            })
            .collect();

        let errors: Vec<Issue> = subgraph_validation_errors
            .iter()
            .filter(|issue| issue.severity == Severity::Error)
            .cloned()
            .collect();
        // TODO propagate hints in new flow
        self.add_issues(subgraph_validation_errors.into_iter());
        if !errors.is_empty() {
            return Err(errors);
        }
        Ok((subgraph_definitions, parsed_schemas))
    }

    /// Maps to upgradeSubgraphsIfNecessary and performs following steps
    ///
    /// 1. Parses raw SDL schemas into Subgraph<Initial>
    /// 2. Adds missing federation definitions to the subgraph schemas
    /// 3. Upgrades federation v1 subgraphs to federation v2 schemas.
    ///    This is a no-op if it is already a federation v2 subgraph.
    async fn experimental_upgrade_subgraphs(
        &mut self,
        subgraphs: Vec<SubgraphDefinition>,
    ) -> Result<Vec<SubgraphDefinition>, Vec<Issue>> {
        let mut errors: Vec<Issue> = vec![];
        let initial: Vec<Subgraph<Initial>> = subgraphs
            .into_iter()
            .map(|s| s.try_into())
            .filter_map(|r| r.map_err(|e: SubgraphError| errors.push(e.into())).ok())
            .collect();
        if !errors.is_empty() {
            return Err(errors);
        }
        let expanded = expand_subgraphs(initial)
            .map_err(|errors| errors.into_iter().map(Issue::from).collect::<Vec<_>>())?;
        upgrade_subgraphs_if_necessary(expanded)
            .map(|subgraphs| {
                subgraphs
                    .into_iter()
                    .map(|s| SubgraphDefinition {
                        sdl: s.schema_string(),
                        name: s.name,
                        url: s.url,
                    })
                    .collect()
            })
            .map_err(|errors| errors.into_iter().map(Issue::from).collect::<Vec<_>>())
    }

    /// Performs all subgraph validations.
    async fn experimental_validate_subgraphs(
        &mut self,
        subgraphs: Vec<SubgraphDefinition>,
    ) -> Result<Vec<SubgraphDefinition>, Vec<Issue>> {
        let mut subgraph_errors = vec![];
        let upgraded: Vec<Subgraph<Upgraded>> = subgraphs
            .into_iter()
            .map(|s| {
                Subgraph::parse(s.name.as_str(), s.url.as_str(), s.sdl.as_str())
                    .and_then(|s| s.assume_expanded())
                    .map(|s| s.assume_upgraded())
            })
            .filter_map(|r| r.map_err(|e| subgraph_errors.push(Issue::from(e))).ok())
            .collect();
        if !subgraph_errors.is_empty() {
            // this should never happen
            return Err(subgraph_errors);
        }
        validate_subgraphs(upgraded)
            .map(|subgraphs| {
                subgraphs
                    .into_iter()
                    .map(|s| SubgraphDefinition {
                        sdl: s.schema_string(),
                        name: s.name,
                        url: s.url,
                    })
                    .collect()
            })
            .map_err(|errors| errors.into_iter().map(Issue::from).collect::<Vec<_>>())
    }

    async fn experimental_merge_subgraphs(
        &mut self,
        subgraphs: Vec<SubgraphDefinition>,
    ) -> Result<CompositionResult, Vec<Issue>> {
        let mut subgraph_errors = vec![];
        let validated: Vec<Subgraph<Validated>> = subgraphs
            .into_iter()
            .map(|s| {
                Subgraph::parse(s.name.as_str(), s.url.as_str(), s.sdl.as_str())
                    .and_then(|s| s.assume_expanded())
                    .map(|s| s.assume_upgraded())
                    .and_then(|s| s.assume_validated())
            })
            .filter_map(|r| r.map_err(|e| subgraph_errors.push(Issue::from(e))).ok())
            .collect();
        if !subgraph_errors.is_empty() {
            // this should never happen
            return Err(subgraph_errors);
        }
        pre_merge_validations(&validated)
            .map_err(|errors| errors.into_iter().map(Issue::from).collect::<Vec<_>>())?;
        let supergraph = merge_subgraphs(validated)
            .map_err(|errors| errors.into_iter().map(Issue::from).collect::<Vec<_>>())?;
        post_merge_validations(&supergraph)
            .map_err(|errors| errors.into_iter().map(Issue::from).collect::<Vec<_>>())?;
        let hints = if supergraph.hints().is_empty() {
            None
        } else {
            Some(
                supergraph
                    .hints()
                    .iter()
                    .map(|h| CompositionHint {
                        message: h.message.clone(),
                        definition: HintCodeDefinition {
                            code: h.code.clone(),
                        },
                        nodes: None,
                    })
                    .collect(),
            )
        };
        Ok(CompositionResult {
            supergraph: supergraph.schema().to_string(),
            hints,
        })
    }

    async fn experimental_validate_satisfiability(
        &mut self,
        supergraph_sdl: &str,
    ) -> Result<SatisfiabilityResult, Vec<Issue>> {
        let supergraph = Supergraph::parse(supergraph_sdl).map_err(|e| vec![Issue::from(e)])?;
        validate_satisfiability(supergraph)
            .map(|s| {
                let hints = if s.hints().is_empty() {
                    None
                } else {
                    Some(
                        s.hints()
                            .iter()
                            .map(|h| CompositionHint {
                                message: h.message.clone(),
                                definition: HintCodeDefinition {
                                    code: h.code.clone(),
                                },
                                nodes: None,
                            })
                            .collect(),
                    )
                };
                SatisfiabilityResult {
                    hints,
                    errors: None,
                }
            })
            .map_err(|errors| errors.into_iter().map(Issue::from).collect::<Vec<_>>())
    }
}

struct SubgraphSchema {
    schema: Schema,
    has_connectors: bool,
}

/// Validate overrides for connector-related subgraphs
///
/// Overrides mess with the supergraph in ways that can be difficult to detect when
/// expanding connectors; the supergraph may omit overridden fields and other shenanigans.
/// To allow for a better developer experience, we check here if any connector-enabled subgraphs
/// have fields overridden.
fn validate_overrides(schemas: HashMap<String, SubgraphSchema>) -> Vec<Issue> {
    let mut override_errors = Vec::new();
    for (subgraph_name, SubgraphSchema { schema, .. }) in &schemas {
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
            .filter(|(_, directive)| {
                // TODO: The directive name for @override could have been aliased
                // at the SDL level, so we'll need to extract the aliased name here instead
                directive.name == "override" || directive.name == "federation__override"
            });

        // Now see if we have any overrides that try to reference connector subgraphs
        for (field, directive) in override_directives {
            // If the override directive does not have a valid `from` field, then there is
            // no point trying to validate it, as later steps will validate the entire schema.
            let Ok(Some(overridden_subgraph_name)) = directive
                .argument_by_name("from", schema)
                .map(|node| node.as_str())
            else {
                continue;
            };

            if schemas
                .get(overridden_subgraph_name)
                .is_some_and(|schema| schema.has_connectors)
            {
                override_errors.push(Issue {
                        code: "OVERRIDE_ON_CONNECTOR".to_string(),
                        message: format!(
                            r#"Field "{}" on subgraph "{}" is trying to override connector-enabled subgraph "{}", which is not yet supported. See https://go.apollo.dev/connectors/limitations#override-is-partially-unsupported"#,
                            field,
                            subgraph_name,
                            overridden_subgraph_name,
                        ),
                        locations: vec![SubgraphLocation {
                            subgraph: Some(String::from(overridden_subgraph_name)),
                            range: directive.line_column_range(&schema.sources),
                        }],
                        severity: Severity::Error,
                    });
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

fn convert_severity(severity: ValidationSeverity) -> Severity {
    match severity {
        ValidationSeverity::Error => Severity::Error,
        ValidationSeverity::Warning => Severity::Warning,
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
