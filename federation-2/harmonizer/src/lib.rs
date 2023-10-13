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
use deno_core::{op, Extension, JsRuntime, Op, OpState, RuntimeOptions, Snapshot};
use std::borrow::Cow;
use std::sync::mpsc::{channel, Sender};

mod js_types;

use js_types::CompositionError;

use apollo_federation_types::build::{
    BuildError, BuildErrors, BuildOutput, BuildResult, SubgraphDefinition,
};

/// The `harmonize` function receives a [`Vec<SubgraphDefinition>`] and invokes JavaScript
/// composition on it, either returning the successful output, or a list of error messages.
pub fn harmonize(subgraph_definitions: Vec<SubgraphDefinition>) -> BuildResult {
    return harmonize_limit(subgraph_definitions, None)
}

/// The `harmonize` function receives a [`Vec<SubgraphDefinition>`] and invokes JavaScript
/// composition on it, either returning the successful output, or a list of error messages.
/// `nodes_limit` limits the number of returns schema nodes to prevent OOM issues
pub fn harmonize_limit(subgraph_definitions: Vec<SubgraphDefinition>, nodes_limit: Option<u32>) -> BuildResult {
    // The snapshot is created in the build_harmonizer.rs script and included in our binary image
    let buffer = include_bytes!(concat!(env!("OUT_DIR"), "/composition.snap"));

    // We'll use this channel to get the results
    let (tx, rx) = channel::<Result<BuildOutput, BuildErrors>>();

    let my_ext = Extension {
        name: env!("CARGO_PKG_NAME"),
        ops: Cow::Borrowed(&[op_composition_result::DECL]),
        op_state_fn: Some(Box::new(move |state| {
            state.put(tx);
        })),
        ..Default::default()
    };

    // Use our snapshot to provision our new runtime
    let options = RuntimeOptions {
        startup_snapshot: Some(Snapshot::Static(buffer)),
        extensions: vec![my_ext],
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
            deno_core::FastString::Owned(format!("nodesLimit = {}", nodes_limit.map(|n| n.to_string()).unwrap_or("null".to_string())).into()),
        )
        .expect("unable to evaluate nodes limit in JavaScript runtime");

    // run the unmodified do_compose.js file, which expects `serviceList` to be set
    runtime
        .execute_script(
            "do_compose",
            deno_core::FastString::Static(include_str!("../bundled/do_compose.js")),
        )
        .expect("unable to invoke composition in JavaScript runtime");

    // wait for a message from `op_composition_result`
    rx.recv().expect("channel remains open")
}

#[op]
fn op_composition_result(state: &mut OpState, value: serde_json::Value) {
    // the JavaScript object can contain an array of errors
    let deserialized_result: Result<Result<BuildOutput, Vec<CompositionError>>, serde_json::Error> =
        serde_json::from_value(value);

    let build_result: Result<BuildOutput, Vec<CompositionError>> = match deserialized_result {
        Ok(build_result) => build_result,
        Err(e) => Err(vec![CompositionError::generic(format!(
            "Something went wrong, this is a bug: {e}"
        ))]),
    };

    let build_result: BuildResult = build_result.map_err(|composition_errors| {
        // we then embed that array of errors into the `BuildErrors` type which is implemented
        // as a single error with each of the underlying errors listed as causes.
        composition_errors
            .iter()
            .map(|err| BuildError::from(err.clone()))
            .collect::<BuildErrors>()
    });

    let sender = state
        .borrow::<Sender<Result<BuildOutput, BuildErrors>>>()
        .clone();
    // send the build result
    sender.send(build_result).expect("channel must be open");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use crate::{harmonize, SubgraphDefinition};

        insta::assert_snapshot!(
            harmonize(vec![
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
            ])
            .unwrap()
            .supergraph_sdl
        );
    }
}
