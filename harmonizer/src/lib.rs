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
#[cfg(not(all(target_os = "macos", target_arch = "x86_64")))]
use deno_core::Snapshot;
use deno_core::{JsRuntime, RuntimeOptions};

mod js_types;

use js_types::CompositionError;

use apollo_federation_types::build::{
    BuildError, BuildErrors, BuildOutput, BuildResult, SubgraphDefinition,
};

// A reasonable default starting limit for our deno heap.
const APOLLO_HARMONIZER_EXPERIMENTAL_V8_INITIAL_HEAP_SIZE_DEFAULT: &str = "256";
// A reasonable default max limit for our deno heap.
const APOLLO_HARMONIZER_EXPERIMENTAL_V8_MAX_HEAP_SIZE_DEFAULT: &str = "1400";

/// The `harmonize` function receives a [`Vec<SubgraphDefinition>`] and invokes JavaScript
/// composition on it, either returning the successful output, or a list of error messages.
pub fn harmonize(subgraph_definitions: Vec<SubgraphDefinition>) -> BuildResult {
    harmonize_limit(subgraph_definitions, None)
}

/// The `harmonize` function receives a [`Vec<SubgraphDefinition>`] and invokes JavaScript
/// composition on it, either returning the successful output, or a list of error messages.
/// `nodes_limit` limits the number of returns schema nodes to prevent OOM issues
pub fn harmonize_limit(
    subgraph_definitions: Vec<SubgraphDefinition>,
    nodes_limit: Option<u32>,
) -> BuildResult {
    let initial_heap_size = std::env::var("APOLLO_HARMONIZER_EXPERIMENTAL_V8_INITIAL_HEAP_SIZE")
        .unwrap_or_else(|_e| {
            APOLLO_HARMONIZER_EXPERIMENTAL_V8_INITIAL_HEAP_SIZE_DEFAULT.to_string()
        });

    let max_heap_size_maybe = std::env::var("APOLLO_HARMONIZER_EXPERIMENTAL_V8_MAX_HEAP_SIZE").ok();
    let max_heap_size_provided = max_heap_size_maybe.is_some();
    let max_heap_size = max_heap_size_maybe
        .unwrap_or_else(|| APOLLO_HARMONIZER_EXPERIMENTAL_V8_MAX_HEAP_SIZE_DEFAULT.to_string());

    // The first flag is argv[0], so provide an ignorable value
    let flags = vec![
        "--ignored".to_string(),
        "--initial_heap_size".to_string(),
        initial_heap_size.to_string(),
        "--max-heap-size".to_string(),
        max_heap_size.to_string(),
    ];

    // Deno will warn us if we supply flags it doesn't recognise.
    // We ignore "--ignored" and report any others as warnings
    let ignored: Vec<_> = deno_core::v8_set_flags(flags)
        .into_iter()
        .filter(|x| x != "--ignored")
        .collect();
    if !ignored.is_empty() {
        panic!("deno ignored these flags: {:?}", ignored);
    }

    // The snapshot is created in the build_harmonizer.rs script and included in our binary image
    #[cfg(not(all(target_os = "macos", target_arch = "x86_64")))]
    let buffer = include_bytes!(concat!(env!("OUT_DIR"), "/composition.snap"));

    #[cfg(not(all(target_os = "macos", target_arch = "x86_64")))]
    let mut runtime = JsRuntime::new(RuntimeOptions {
        startup_snapshot: Some(Snapshot::Static(buffer)),
        ..Default::default()
    });

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let mut runtime = {
        let mut runtime = JsRuntime::new(RuntimeOptions {
            ..Default::default()
        });

        // The runtime automatically contains a Deno.core object with several
        // functions for interacting with it.
        let runtime_str = include_str!("../bundled/runtime.js");
        runtime
            .execute_script("<init>", deno_core::FastString::Owned(runtime_str.into()))
            .expect("unable to initialize router bridge runtime environment");

        // Load the composition library.
        let bridge_str = include_str!("../bundled/composition_bridge.js");
        runtime
            .execute_script("bridge.js", deno_core::FastString::Owned(bridge_str.into()))
            .expect("unable to evaluate bridge module");
        runtime
    };

    // if max_heap_size was not set, we resize the heap every time
    // we approach the limit. This is a tradeoff as it might cause
    // an instance to run out of physical memory.
    if !max_heap_size_provided {
        // Add a callback that expands our heap by 1.25 each time
        // it is invoked. There is no limit, since we rely on the
        // execution environment (OS) to provide that.
        let name = "harmonize".to_string();
        runtime.add_near_heap_limit_callback(move |current, initial| {
            let new = current * 5 / 4;
            tracing::info!(
                "deno heap expansion({}): initial: {}, current: {}, new: {}",
                name,
                initial,
                current,
                new
            );
            new
        });
    }

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
