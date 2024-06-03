/*!
# Harmonizer

This _harmonizer_ offers the ability to invoke a bundled version of the
JavaScript library, [`@apollo/composition`], which _composes_ multiple subgraphs
into a supergraph.

The bundled version of the federation library that is included is a JavaScript
Immediately Invoked Function Expression ([IIFE]) that is created by running the
[esbuild] bundler on the `@apollo/composition` package.

When the [`harmonize`] function that this crate provides is called with a
[`ServiceList`] (which is synonymous with the terminology and service list
notion that exists within the JavaScript composition library), this crate uses
[`deno_core`] to invoke the JavaScript within V8. This is ultimately
accomplished using [`rusty_v8`]'s V8 bindings to V8.

While we intend for a future version of composition to be done natively within
Rust, this allows us to provide a more stable transition using an already stable
composition implementation while we work toward something else.

[`@apollo/composition`]: https://npm.im/@apollo/composition
[IIFE]: https://developer.mozilla.org/en-US/docs/Glossary/IIFE
[esbuild]: https://esbuild.github.io/
[`deno_core`]: https://crates.io/crates/deno_core
[`rusty_v8`]: https://crates.io/crates/rusty_v8
*/

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations, nonstandard_style)]
#![warn(missing_docs, future_incompatible, unreachable_pub, rust_2018_idioms)]
use apollo_composition::{Composer, Issue, Severity, SubgraphLocation};
use deno_core::{JsRuntime, RuntimeOptions, Snapshot};

mod js_types;

use js_types::CompositionError;

use apollo_federation_types::build::{
    BuildError, BuildErrorNode, BuildErrorNodeLocationToken, BuildErrors, BuildHint, BuildOutput,
    BuildResult, SubgraphDefinition,
};

/// An implementation of `apollo_composition::JavaScriptExecutor` that uses Deno to execute
/// the TypeScript composition library.
#[derive(Debug, Default)]
pub struct Harmonizer {
    /// If merging (JavaScript composition) succeeds, this will contain the supergraph SDL.
    pub supergraph_sdl: Option<String>,
    /// Hints are suggestions to improve subgraphs, but are not errors.
    pub hints: Vec<BuildHint>,
    /// Errors are issues that prevent successful composition.
    pub errors: BuildErrors,
}

impl Composer for Harmonizer {
    async fn compose_services(
        &mut self,
        subgraph_definitions: Vec<SubgraphDefinition>,
    ) -> Option<&str> {
        match harmonize_limit(subgraph_definitions, None) {
            Ok(output) => {
                self.supergraph_sdl = Some(output.supergraph_sdl);
                self.hints.extend(output.hints);
            }
            Err(errors) => {
                self.errors.extend(errors);
            }
        }
        self.supergraph_sdl.as_deref()
    }

    fn add_issues<Source: Iterator<Item = Issue>>(&mut self, issues: Source) {
        for issue in issues {
            match issue.severity {
                Severity::Error => self.errors.push(BuildError::composition_error(
                    Some(issue.code.to_string()),
                    Some(issue.message),
                    Some(transform_locations(issue.locations)),
                    None,
                )),
                Severity::Warning => self.hints.push(BuildHint {
                    message: issue.message,
                    code: Some(issue.code.to_string()),
                    nodes: Some(transform_locations(issue.locations)),
                    omitted_nodes_count: None,
                    other: Default::default(),
                }),
            }
        }
    }
}

fn transform_locations(locations: Vec<SubgraphLocation>) -> Vec<BuildErrorNode> {
    locations
        .into_iter()
        .map(|location| BuildErrorNode {
            subgraph: Some(location.subgraph),
            source: None,
            start: Some(BuildErrorNodeLocationToken {
                line: Some(location.start.line + 1),
                column: Some(location.start.column + 1),
                start: None,
                end: None,
            }),
            end: Some(BuildErrorNodeLocationToken {
                line: Some(location.end.line + 1),
                column: Some(location.end.column + 1),
                start: None,
                end: None,
            }),
        })
        .collect()
}

/// The `harmonize` function receives a [`Vec<SubgraphDefinition>`] and invokes JavaScript
/// composition on it, either returning the successful output, or a list of error messages.
/// `nodes_limit` limits the number of returns schema nodes to prevent OOM issues
pub fn harmonize_limit(
    subgraph_definitions: Vec<SubgraphDefinition>,
    nodes_limit: Option<u32>,
) -> BuildResult {
    // The snapshot is created in the build_harmonizer.rs script and included in our binary image
    let buffer = include_bytes!(concat!(env!("OUT_DIR"), "/composition.snap"));

    // Use our snapshot to provision our new runtime
    let options = RuntimeOptions {
        startup_snapshot: Some(Snapshot::Static(buffer)),
        ..Default::default()
    };
    let mut runtime = JsRuntime::new(options);

    // convert the subgraph definitions into JSON
    let service_list_javascript = format!(
        "serviceList = {}",
        serde_json::to_string(&subgraph_definitions)
            .expect("unable to serialize service list into JavaScript runtime")
    );

    // store the subgraph definition JSON in the `serviceList` variable
    runtime
        .execute_script(
            "<set_service_list>",
            deno_core::FastString::Owned(service_list_javascript.into()),
        )
        .expect("unable to evaluate service list in JavaScript runtime");

    // store the nodes_limit variable in the nodesLimit variable
    runtime
        .execute_script(
            "<set_nodes_limit>",
            deno_core::FastString::Owned(
                format!(
                    "nodesLimit = {}",
                    nodes_limit
                        .map(|n| n.to_string())
                        .unwrap_or("null".to_string())
                )
                .into(),
            ),
        )
        .expect("unable to evaluate nodes limit in JavaScript runtime");

    // run the unmodified do_compose.js file, which expects `serviceList` to be set
    match runtime.execute_script(
        "do_compose",
        deno_core::FastString::Static(include_str!("../bundled/do_compose.js")),
    ) {
        Ok(execute_result) => {
            let scope = &mut runtime.handle_scope();
            let local = deno_core::v8::Local::new(scope, execute_result);
            match deno_core::serde_v8::from_v8::<Result<BuildOutput, Vec<CompositionError>>>(
                scope, local,
            ) {
                Ok(Ok(output)) => Ok(output),
                Ok(Err(errors)) => {
                    let mut build_errors = BuildErrors::new();
                    for error in errors {
                        build_errors.push(error.into());
                    }
                    Err(build_errors)
                }
                Err(e) => {
                    let mut errors = BuildErrors::new();
                    errors.push(BuildError::composition_error(
                        None,
                        Some(format!("Unable to deserialize composition result: {}", e)),
                        None,
                        None,
                    ));
                    Err(errors)
                }
            }
        }
        Err(e) => {
            let mut errors = BuildErrors::new();
            errors.push(BuildError::composition_error(
                None,
                Some(format!(
                    "Error invoking composition in JavaScript runtime: {}",
                    e
                )),
                None,
                None,
            ));
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        insta::assert_snapshot!(
            harmonize_limit(
                vec![
                    SubgraphDefinition::new(
                        "users",
                        "undefined",
                        "
            type User @key(fields: \"id\") {
              id: ID
              name: String
            }

            type Query {
              users: [User!]
            }
          "
                    ),
                    SubgraphDefinition::new(
                        "movies",
                        "undefined",
                        "
            type Movie {
              title: String
              name: String
            }

            type User @key(fields: \"id\") {
              id: ID
              favorites: [Movie!]
            }

            type Query {
              movies: [Movie!]
            }
          "
                    )
                ],
                None
            )
            .unwrap()
            .supergraph_sdl
        );
    }
}
