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
use deno_core::{op_sync, JsRuntime, RuntimeOptions, Snapshot};
use std::sync::mpsc::channel;

mod js_types;

use js_types::CompositionError;

use apollo_federation_types::build::{BuildError, BuildOutput, BuildResult, SubgraphDefinition};

/// The `harmonize` function receives a [`Vec<SubgraphDefinition>`] and invokes JavaScript
/// composition on it, either returning the successful output, or a list of error messages.
pub fn harmonize(subgraph_definitions: Vec<SubgraphDefinition>) -> BuildResult {
    // The snapshot is created in the build_harmonizer.rs script and included in our binary image
    let buffer = include_bytes!(concat!(env!("OUT_DIR"), "/composition.snap"));

    // Use our snapshot to provision our new runtime
    let options = RuntimeOptions {
        startup_snapshot: Some(Snapshot::Static(buffer)),
        ..Default::default()
    };
    let mut runtime = JsRuntime::new(options);

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

            tx.send(js_composition_result.map(BuildOutput::new).map_err(|errs| {
                errs.iter()
                    .map(|err| BuildError::from(err.clone()))
                    .collect()
            }))
            .expect("channel must be open");

            Ok(serde_json::json!(null))

            // Don't return anything to JS
        }),
    );
    runtime.sync_ops_cache();

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
        .execute_script("do_compose.js", include_str!("../bundled/do_compose.js"))
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
