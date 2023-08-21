use crate::error::Error;
/// Wraps creating the Deno Js runtime collecting parameters and executing a script.
use deno_core::{
    anyhow::{anyhow, Error as AnyError},
    op, Extension, JsRuntime, Op, OpState, RuntimeOptions, Snapshot,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::{
    borrow::Cow,
    sync::mpsc::{channel, Sender},
};

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

    pub(crate) fn execute<Ok: DeserializeOwned + 'static>(
        &self,
        name: &'static str,
        source: &'static str,
    ) -> Result<Ok, Error> {
        // We'll use this channel to get the results
        let (tx, rx) = channel::<Result<Ok, Error>>();

        let happy_tx = tx.clone();

        let my_ext = Extension {
            name: "router_bridge",
            ops: Cow::Borrowed(&[deno_result::<Ok>::DECL]),
            op_state_fn: Some(Box::new(move |state: &mut OpState| {
                state.put(happy_tx);
            })),
            ..Default::default()
        };

        let mut runtime = self.build_js_runtime(my_ext);

        for parameter in self.parameters.iter() {
            runtime
                .execute_script(
                    parameter.0,
                    deno_core::FastString::Owned(parameter.1.clone().into()),
                )
                .expect("unable to evaluate service list in JavaScript runtime");
        }

        // We are sending the error through the channel already
        let _ = runtime
            .execute_script(name, deno_core::FastString::Static(source))
            .map_err(|e| {
                let message =
                    format!("unable to invoke `{name}` in JavaScript runtime \n error: \n {e:?}");

                tx.send(Err(Error::DenoRuntime(message)))
                    .expect("channel must be open");

                e
            });

        rx.recv().expect("channel remains open")
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

            fn check_unstable(&self, _state: &deno_core::OpState, _api_name: &'static str) {
                unreachable!("not needed in the planner")
            }
        }

        let mut js_runtime = JsRuntime::new(RuntimeOptions {
            extensions: vec![
                deno_webidl::deno_webidl::init_ops(),
                deno_console::deno_console::init_ops(),
                deno_url::deno_url::init_ops(),
                deno_web::deno_web::init_ops::<Permissions>(Default::default(), Default::default()),
                deno_crypto::deno_crypto::init_ops(None),
                my_ext,
            ],
            startup_snapshot: Some(Snapshot::Static(buffer)),
            ..Default::default()
        });

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

#[op]
fn deno_result<Response>(state: &mut OpState, payload: Response) -> Result<(), AnyError>
where
    Response: DeserializeOwned + 'static,
{
    // we're cloning here because we don't wanna keep the borrow across an await point
    let sender = state.borrow::<Sender<Result<Response, Error>>>().clone();
    sender
        .send(Ok(payload))
        .map_err(|e| anyhow!("couldn't send response {e}"))
}
