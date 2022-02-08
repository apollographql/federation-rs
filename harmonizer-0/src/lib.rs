/*!
# Harmonizer

This _harmonizer_ offers the ability to invoke a bundled version of the
JavaScript library, [`@apollo/federation`], which _composes_ multiple subgraphs
into a supergraph.

The bundled version of the federation library that is included is a JavaScript
Immediately Invoked Function Expression ([IIFE]) that is created by running the
[esbuild] bundler on the `@apollo/federation` package.

When the [`harmonize`] function that this crate provides is called with a
[`ServiceList`] (which is synonymous with the terminology and service list
notion that exists within the JavaScript composition library), this crate uses
[`deno_core`] to invoke the JavaScript within V8.  This is ultimately
accomplished using [`rusty_v8`]'s V8 bindings to V8.

While we intend for a future version of composition to be done natively within
Rust, this allows us to provide a more stable transition using an already stable
composition implementation while we work toward something else.

[`@apollo/federation`]: https://npm.im/@apollo/federation
[IIFE]: https://developer.mozilla.org/en-US/docs/Glossary/IIFE
[esbuild]: http://esbuild.github.io/
[`deno_core`]: https://crates.io/crates/deno_core
[`rusty_v8`]: https://crates.io/crates/rusty_v8
*/

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations, nonstandard_style)]
#![warn(missing_docs, future_incompatible, unreachable_pub, rust_2018_idioms)]
use deno_core::{op_sync, JsRuntime};
use std::sync::mpsc::channel;

mod js_types;

use js_types::CompositionError;

use apollo_federation_types::build::{BuildError, BuildOutput, BuildResult, SubgraphDefinition};

/// The `harmonize` function receives a [`Vec<SubgraphDefinition>`] and invokes JavaScript
/// composition on it, either returning the successful output, or a list of error messages.
pub fn harmonize(subgraph_definitions: Vec<SubgraphDefinition>) -> BuildResult {
    // Initialize a runtime instance
    let mut runtime = JsRuntime::new(Default::default());

    // We'll use this channel to get the results
    let (tx, rx) = channel();

    // Register an operation called "op_composition_result"
    // that will execute the "op_sync" function when it is called
    // from JavaScript with Deno.core.opSync('op_composition_result', result);
    runtime.register_op(
        "op_composition_result",
        op_sync(move |_state, value, _zero_copy: ()| {
            let js_composition_result: Result<String, Vec<CompositionError>> =
                serde_json::from_value(value)
                    .expect("could not deserialize composition result from JS.");

            tx.send(
                js_composition_result
                    .map(|supergraph_sdl| BuildOutput::new(&supergraph_sdl))
                    .map_err(|errs| {
                        errs.iter()
                            .map(|err| BuildError::from(err.clone()))
                            .collect()
                    }),
            )
            .expect("channel must be open");

            Ok(serde_json::json!(null))

            // Don't return anything to JS
        }),
    );

    runtime.sync_ops_cache();

    // The runtime automatically contains a Deno.core object with several
    // functions for interacting with it.
    runtime
        .execute_script(
            "<init>",
            r#"
// First we initialize the operations cache.
// This maps op names to their id's.
Deno.core.ops();

function done(result) {
  Deno.core.opSync('op_composition_result', result);
}

// We build some of the preliminary objects that our esbuilt package is
// expecting to be present in the environment.
// 'process' is a Node.js ism.  We rely on process.env.NODE_ENV, in
// particular, to determine whether or not we are running in a debug
// mode.  For the purposes of harmonizer, we don't gain anything from
// running in such a mode.
process = { env: { "NODE_ENV": "production" }};
// Some JS runtime implementation specific bits that we rely on that
// need to be initialized as empty objects.
global = {};
exports = {};
"#,
        )
        .expect("unable to initialize composition runtime environment");

    // Load the composition library.
    runtime
        .execute_script("composition.js", include_str!("../dist/composition.js"))
        .expect("unable to evaluate composition module");

    // convert the subgraph definitions into JSON
    let service_list_javascript = format!(
        "serviceList = {}",
        serde_json::to_string(&subgraph_definitions)
            .expect("unable to serialize service list into JavaScript runtime")
    );

    // store the subgraph definition JSON in the `serviceList` variable
    runtime
        .execute_script("<set_service_list>", &service_list_javascript)
        .expect("unable to evaluate service list in JavaScript runtime");

    // run the unmodified do_compose.js file, which expects `serviceList` to be set
    runtime
        .execute_script("do_compose.js", include_str!("../deno/do_compose.js"))
        .expect("unable to invoke composition in JavaScript runtime");

    // wait for a message from `op_composition_result`
    rx.recv().expect("channel remains open")
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

            extend type User {
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
