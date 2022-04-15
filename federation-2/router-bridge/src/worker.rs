use crate::error::Error;
use async_channel::{bounded, Receiver, Sender};
use deno_core::{op, Extension, JsRuntime, OpState, RuntimeOptions, Snapshot};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::JoinHandle;
use tokio::sync::{oneshot, Mutex};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
struct JsonPayload {
    id: Uuid,
    payload: serde_json::Value,
}

pub(crate) struct JsWorker {
    response_senders: Arc<Mutex<HashMap<Uuid, oneshot::Sender<serde_json::Value>>>>,
    response_receivers: Arc<Mutex<HashMap<Uuid, oneshot::Receiver<serde_json::Value>>>>,
    sender: Sender<JsonPayload>,
    handle: Option<JoinHandle<()>>,
}

impl JsWorker {
    pub(crate) fn new(worker_source_code: &'static str) -> Self {
        let response_senders: Arc<Mutex<HashMap<Uuid, oneshot::Sender<serde_json::Value>>>> =
            Default::default();

        let cloned_senders = response_senders.clone();

        let (response_sender, receiver) = bounded::<JsonPayload>(10_000);
        let (sender, request_receiver) = bounded::<JsonPayload>(10_000);

        tokio::spawn(async move {
            while let Ok(json_payload) = receiver.recv().await {
                let sender = cloned_senders
                    .lock()
                    .await
                    .remove(&json_payload.id)
                    .expect("TODO");
                let _ = sender.send(json_payload.payload).map_err(|e| {
                    tracing::error!("jsworker: couldn't send json response: {:?}", e);
                });
            }
            tracing::debug!("deno runtime shutdown successfully");
        });

        let handle = std::thread::spawn(move || {
            let my_ext = Extension::builder()
                .ops(vec![send::decl(), receive::decl()])
                .state(move |state| {
                    state.put(response_sender.clone());
                    state.put(request_receiver.clone());

                    Ok(())
                })
                .build();
            // Initialize a runtime instance
            let buffer = include_bytes!(concat!(env!("OUT_DIR"), "/query_runtime.snap"));

            let mut js_runtime = JsRuntime::new(RuntimeOptions {
                extensions: vec![my_ext],
                startup_snapshot: Some(Snapshot::Static(buffer)),
                ..Default::default()
            });

            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let future = async move {
                js_runtime
                    .execute_script("worker.js", worker_source_code)
                    .unwrap();
                js_runtime.run_event_loop(false).await
            };
            runtime.block_on(future).unwrap();
        });

        Self {
            sender,
            handle: Some(handle),
            response_receivers: Default::default(),
            response_senders,
        }
    }

    pub(crate) async fn request<Request, Response>(
        &self,
        command: Request,
    ) -> Result<Response, Error>
    where
        Request: Serialize + Send + Debug + 'static,
        Response: DeserializeOwned + Send + Debug + 'static,
    {
        let id = self
            .send(command)
            .await
            .map_err(|e| Error::DenoRuntime(format!("couldn't send request {e}")))?;
        self.receive(id)
            .await
            .map_err(|e| Error::DenoRuntime(format!("request: couldn't receive response {e}")))
    }

    pub(crate) async fn send<Request>(&self, request: Request) -> Result<Uuid, Error>
    where
        Request: Serialize + Send + Debug + 'static,
    {
        let id = Uuid::new_v4();

        let (sender, receiver) = oneshot::channel();
        {
            self.response_senders.lock().await.insert(id, sender);
            self.response_receivers.lock().await.insert(id, receiver);
        }
        let json_payload = JsonPayload {
            id,
            payload: serde_json::to_value(request).map_err(|e| Error::ParameterSerialization {
                message: format!("deno: couldn't serialize request : `{:?}`", e),
                name: "request".to_string(),
            })?,
        };

        self.sender
            .send(json_payload)
            .await
            .map_err(|e| Error::DenoRuntime(format!("send: couldn't send request {e}")))?;
        Ok(id)
    }

    async fn receive<Response>(&self, id: Uuid) -> Result<Response, Error>
    where
        Response: DeserializeOwned + Send + Debug + 'static,
    {
        let receiver = self
            .response_receivers
            .lock()
            .await
            .remove(&id)
            .expect("couldn't find id in response_receivers");
        let payload = receiver.await.map_err(|e| {
            Error::DenoRuntime(format!("request: couldn't receive response: {:?}", e))
        })?;

        serde_json::from_value(payload).map_err(|e| Error::ParameterDeserialization {
            message: format!("deno: couldn't deserialize response : `{:?}`", e),
            id: format!("id: {id}"),
        })
    }

    fn quit(&mut self) -> Result<(), Error> {
        if let Some(handle) = self.handle.take() {
            handle.join().map_err(|_| {
                Error::DenoRuntime("couldn't wait for JsRuntime to finish".to_string())
            })
        } else {
            Ok(())
        }
    }
}

impl Drop for JsWorker {
    fn drop(&mut self) {
        self.quit().unwrap_or_else(|e| eprintln!("{}", e));
    }
}

#[op]
async fn send(state: Rc<RefCell<OpState>>, payload: JsonPayload) -> Result<(), anyhow::Error> {
    let sender = {
        let state = state.borrow();
        // we're cloning here because we don't wanna keep the borrow across an await point
        state.borrow::<Sender<JsonPayload>>().clone()
    };

    sender
        .send(payload)
        .await
        .map_err(|e| anyhow::anyhow!("couldn't send response {e}"))
}

#[op]
async fn receive(state: Rc<RefCell<OpState>>) -> Result<JsonPayload, anyhow::Error> {
    let state = state.borrow();
    let receiver = state.borrow::<Receiver<JsonPayload>>();
    receiver
        .recv()
        .await
        .map_err(|e| anyhow::anyhow!("op_receive: couldn't send response {e}"))
}
