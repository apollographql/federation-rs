use crate::error::Error;
/// Wraps creating the Deno Js runtime collecting parameters and executing a script.
use deno_core::{
    anyhow::{anyhow, Error as AnyError},
    op, Extension, JsRuntime, OpState, RuntimeOptions, Snapshot,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::mpsc::{channel, Sender};

pub(crate) struct Js {
    parameters: Vec<(&'static str, String)>,
}

impl Js {
    pub(crate) fn new() -> Js {
        Js {
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

        let my_ext = Extension::builder()
            .ops(vec![deno_result::decl::<Ok>()])
            .state(move |state| {
                state.put(happy_tx.clone());
                Ok(())
            })
            .build();

        // The snapshot is created in our build.rs script and included in our binary image
        let buffer = include_bytes!(concat!(env!("OUT_DIR"), "/query_runtime.snap"));

        // Use our snapshot to provision our new runtime
        let options = RuntimeOptions {
            extensions: vec![my_ext],
            startup_snapshot: Some(Snapshot::Static(buffer)),
            ..Default::default()
        };
        let mut runtime = JsRuntime::new(options);

        for parameter in self.parameters.iter() {
            runtime
                .execute_script(format!("<{}>", parameter.0).as_str(), &parameter.1)
                .expect("unable to evaluate service list in JavaScript runtime");
        }

        // We are sending the error through the channel already
        let _ = runtime.execute_script(name, source).map_err(|e| {
            let message = format!(
                "unable to invoke `{name}` in JavaScript runtime \n error: \n {:?}",
                e
            );

            tx.send(Err(Error::DenoRuntime(message)))
                .expect("channel must be open");

            e
        });

        rx.recv().expect("channel remains open")
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
