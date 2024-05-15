use crate::error::Error;
/// Wraps creating the Deno Js runtime collecting parameters and executing a script.
use deno_core::{Extension, JsRuntime, RuntimeOptions};
use serde::de::DeserializeOwned;
use serde::Serialize;

// A reasonable default starting limit for our deno heap.
const APOLLO_ROUTER_BRIDGE_EXPERIMENTAL_V8_INITIAL_HEAP_SIZE_DEFAULT: &str = "256";

pub(crate) struct Js {
    name: String,
    parameters: Vec<(&'static str, String)>,
}

impl Js {
    pub(crate) fn new(name: String) -> Js {
        Js {
            name,
            parameters: Vec::new(),
        }
    }

    pub(crate) fn with_parameter<T: Serialize>(
        mut self,
        name: &'static str,
        param: T,
    ) -> Result<Js, Error> {
        let serialized = format!(
            "{} = {}",
            name,
            serde_json::to_string(&param).map_err(|error| Error::ParameterSerialization {
                name: name.to_string(),
                message: error.to_string()
            })?
        );
        self.parameters.push((name, serialized));
        Ok(self)
    }

    pub(crate) fn execute<OkResult: DeserializeOwned + 'static>(
        &self,
        name: &'static str,
        source: &'static str,
    ) -> Result<OkResult, Error> {
        let noop_ext = Extension {
            name: env!("CARGO_PKG_NAME"),
            ..Default::default()
        };

        let mut runtime = self.build_js_runtime(noop_ext);

        for parameter in self.parameters.iter() {
            runtime
                .execute_script(parameter.0, parameter.1.clone())
                .expect("unable to evaluate service list in JavaScript runtime");
        }

        match runtime.execute_script(name, source) {
            Ok(execute_result) => {
                let scope = &mut runtime.handle_scope();
                let local = deno_core::v8::Local::new(scope, execute_result);
                match deno_core::serde_v8::from_v8::<OkResult>(scope, local) {
                    Ok(result) => Ok(result),
                    Err(e) => Err(Error::DenoRuntime(format!(
                        "unable to deserialize result of `{name}` in JavaScript runtime \n error: \n {e:?}"
                    ))),
                }
            }
            Err(e) => {
                let message =
                    format!("unable to invoke `{name}` in JavaScript runtime \n error: \n {e:?}");
                Err(Error::DenoRuntime(message))
            }
        }
    }

    pub(crate) fn build_js_runtime(&self, my_ext: Extension) -> JsRuntime {
        // Initialize a runtime instance
        let buffer = include_bytes!(concat!(env!("OUT_DIR"), "/query_runtime.snap"));

        let heap_size =
            match std::env::var("APOLLO_ROUTER_BRIDGE_EXPERIMENTAL_V8_INITIAL_HEAP_SIZE") {
                Ok(v) => v,
                Err(_e) => {
                    APOLLO_ROUTER_BRIDGE_EXPERIMENTAL_V8_INITIAL_HEAP_SIZE_DEFAULT.to_string()
                }
            };

        // The first flag is argv[0], so provide an ignorable value
        let flags = vec![
            "--ignored".to_string(),
            "--max-heap-size".to_string(),
            heap_size,
        ];

        // Deno will warn us if we supply flags it doesn't recognise.
        // We ignore "--ignored" and report any others as warnings
        let ignored: Vec<_> = deno_core::v8_set_flags(flags)
            .into_iter()
            .filter(|x| x != "--ignored")
            .collect();
        if !ignored.is_empty() {
            tracing::warn!("deno ignored these flags: {:?}", ignored);
        }

        #[derive(Clone)]
        struct Permissions;

        impl deno_web::TimersPermission for Permissions {
            fn allow_hrtime(&mut self) -> bool {
                // not needed in the planner
                false
            }
        }

        #[cfg(not(all(target_os = "macos", target_arch = "x86_64")))]
        let mut js_runtime = JsRuntime::new(RuntimeOptions {
            extensions: vec![
                deno_webidl::deno_webidl::init_ops(),
                deno_console::deno_console::init_ops(),
                deno_url::deno_url::init_ops(),
                deno_web::deno_web::init_ops::<Permissions>(Default::default(), Default::default()),
                deno_crypto::deno_crypto::init_ops(None),
                my_ext,
            ],
            startup_snapshot: Some(buffer),
            ..Default::default()
        });

        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        let mut js_runtime = {
            let mut js_runtime = JsRuntime::new(RuntimeOptions {
                extensions: vec![
                    deno_webidl::deno_webidl::init_ops(),
                    deno_console::deno_console::init_ops(),
                    deno_url::deno_url::init_ops(),
                    deno_web::deno_web::init_ops::<Permissions>(
                        Default::default(),
                        Default::default(),
                    ),
                    deno_crypto::deno_crypto::init_ops(None),
                    my_ext,
                ],
                ..Default::default()
            });

            // The runtime automatically contains a Deno.core object with several
            // functions for interacting with it.
            let runtime_str = include_str!("../bundled/runtime.js");
            js_runtime
                .execute_script("<init>", runtime_str)
                .expect("unable to initialize router bridge runtime environment");

            // Load the composition library.
            let bridge_str = include_str!("../bundled/bridge.js");
            js_runtime
                .execute_script("bridge.js", bridge_str)
                .expect("unable to evaluate bridge module");
            js_runtime
        };

        // Add a callback that expands our heap by 1.25 each time
        // it is invoked. There is no limit, since we rely on the
        // execution environment (OS) to provide that.
        let name = self.name.clone();
        js_runtime.add_near_heap_limit_callback(move |current, initial| {
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
        js_runtime
    }
}
