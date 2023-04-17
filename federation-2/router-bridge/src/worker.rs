use crate::error::Error;
use async_channel::{bounded, Receiver, Sender};
use deno_core::{op, Extension, OpState};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::hash::Hasher;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::JoinHandle;
use tokio::sync::{oneshot, Mutex};

#[derive(Serialize, Deserialize, Debug)]
struct JsonPayload {
    id: String,
    payload: serde_json::Value,
}

pub(crate) struct JsWorker {
    response_senders: Arc<Mutex<HashMap<String, oneshot::Sender<serde_json::Value>>>>,
    response_receivers: Arc<Mutex<HashMap<String, oneshot::Receiver<serde_json::Value>>>>,
    sender: Sender<JsonPayload>,
    handle: Option<JoinHandle<()>>,
    unsent_plans: Arc<Mutex<HashMap<String, serde_json::Value>>>,
}

impl JsWorker {
    pub(crate) fn new(worker_source_code: &'static str) -> Self {
        let response_senders: Arc<Mutex<HashMap<String, oneshot::Sender<serde_json::Value>>>> =
            Default::default();

        let cloned_senders = response_senders.clone();

        let (response_sender, receiver) = bounded::<JsonPayload>(10_000);
        let (sender, request_receiver) = bounded::<JsonPayload>(10_000);

        let unsent_plans = Arc::new(Mutex::new(HashMap::new()));
        let my_unsent_plans = unsent_plans.clone();

        tokio::spawn(async move {
            while let Ok(json_payload) = receiver.recv().await {
                if let Some(sender) = cloned_senders.lock().await.remove(&json_payload.id) {
                    if let Err(e) = sender.send(json_payload.payload.clone()) {
                        // Keep our plan in our failed plan cache. Someone else might want it.
                        tracing::error!("jsworker: couldn't send json response: {:?}", e);
                        my_unsent_plans
                            .lock()
                            .await
                            .insert(json_payload.id, json_payload.payload);
                    }
                } else {
                    tracing::error!(
                        "jsworker: couldn't find sender for payload id {}",
                        &json_payload.id
                    );
                }
            }
            tracing::debug!("deno runtime shutdown successfully");
        });

        let handle = std::thread::spawn(move || {
            let my_ext = Extension::builder("router_bridge_worker")
                .ops(vec![
                    send::decl(),
                    receive::decl(),
                    log_trace::decl(),
                    log_debug::decl(),
                    log_info::decl(),
                    log_warn::decl(),
                    log_error::decl(),
                ])
                .state(move |state| {
                    state.put(response_sender.clone());
                    state.put(request_receiver.clone());

                    Ok(())
                })
                .build();

            let mut js_runtime =
                crate::js::Js::new("query planner".to_string()).build_js_runtime(my_ext);

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
            unsent_plans,
        }
    }

    pub(crate) async fn request<Request, Response>(
        &self,
        command: Request,
    ) -> Result<Response, Error>
    where
        Request: std::hash::Hash + Serialize + Send + Debug + 'static,
        Response: DeserializeOwned + Send + Debug + 'static,
    {
        // Let's see if we already have this query plan in our failed delivery cache
        let mut hasher = DefaultHasher::new();
        command.hash(&mut hasher);
        // JavaScript can't process 64 bit numbers, so convert our hash to a string...
        let id = hasher.finish().to_string();

        if let Some(payload) = self.unsent_plans.lock().await.remove(&id) {
            serde_json::from_value(payload).map_err(|e| Error::ParameterDeserialization {
                message: format!("deno: couldn't deserialize response : `{e:?}`"),
                id,
            })
        } else {
            self.send(Some(id.clone()), command)
                .await
                .map_err(|e| Error::DenoRuntime(format!("couldn't send request {e}")))?;
            self.receive(id)
                .await
                .map_err(|e| Error::DenoRuntime(format!("request: couldn't receive response {e}")))
        }
    }

    pub(crate) async fn send<Request>(
        &self,
        id_opt: Option<String>,
        request: Request,
    ) -> Result<String, Error>
    where
        Request: std::hash::Hash + Serialize + Send + Debug + 'static,
    {
        let id = match id_opt {
            Some(id) => id,
            None => {
                let mut hasher = DefaultHasher::new();
                request.hash(&mut hasher);
                // JavaScript can't process 64 bit numbers, so convert our hash to a string...
                hasher.finish().to_string()
            }
        };

        let (sender, receiver) = oneshot::channel();
        {
            self.response_senders
                .lock()
                .await
                .insert(id.clone(), sender);
            self.response_receivers
                .lock()
                .await
                .insert(id.clone(), receiver);
        }
        let json_payload = JsonPayload {
            id: id.clone(),
            payload: serde_json::to_value(request).map_err(|e| Error::ParameterSerialization {
                message: format!("deno: couldn't serialize request : `{e:?}`"),
                name: "request".to_string(),
            })?,
        };

        self.sender
            .send(json_payload)
            .await
            .map_err(|e| Error::DenoRuntime(format!("send: couldn't send request {e}")))?;
        Ok(id)
    }

    async fn receive<Response>(&self, id: String) -> Result<Response, Error>
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
            Error::DenoRuntime(format!("request: couldn't receive response: {e:?}"))
        })?;

        serde_json::from_value(payload).map_err(|e| Error::ParameterDeserialization {
            message: format!("deno: couldn't deserialize response : `{e:?}`"),
            id,
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
        self.quit().unwrap_or_else(|e| eprintln!("{e}"));
    }
}

// Logging capabilities
#[op]
fn log_trace(_: &mut OpState, message: String) -> Result<(), anyhow::Error> {
    tracing::trace!("{message}");
    Ok(())
}

#[op]
fn log_debug(_: &mut OpState, message: String) -> Result<(), anyhow::Error> {
    tracing::debug!("{message}");
    Ok(())
}

#[op]
fn log_info(_: &mut OpState, message: String) -> Result<(), anyhow::Error> {
    tracing::info!("{message}");
    Ok(())
}

#[op]
fn log_warn(_: &mut OpState, message: String) -> Result<(), anyhow::Error> {
    tracing::warn!("{message}");
    Ok(())
}

#[op]
fn log_error(_: &mut OpState, message: String) -> Result<(), anyhow::Error> {
    tracing::error!("{message}");
    Ok(())
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
    let receiver = {
        let state = state.borrow();
        state.borrow::<Receiver<JsonPayload>>().clone()
    };

    receiver
        .recv()
        .await
        .map_err(|e| anyhow::anyhow!("op_receive: couldn't send response {e}"))
}

#[cfg(test)]
mod worker_tests {
    use super::JsWorker;
    use serde::{Deserialize, Serialize};

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn logging_works() {
        let expected_present_logs = [
            "TRACE router_bridge::worker: this is a Trace level log",
            "DEBUG router_bridge::worker: this is a Debug level log",
            "INFO router_bridge::worker: this is an Info level log",
            "WARN router_bridge::worker: this is a Warn level log",
            "ERROR router_bridge::worker: this is an Error level log",
        ];
        run_logger().await;
        logs_assert(|lines: &[&str]| {
            for log in expected_present_logs {
                assert!(
                    lines.iter().any(|line| line.ends_with(log)),
                    "couldn't find log `{}` in the traced logs:\n{}",
                    log,
                    lines.join("\n")
                );
            }

            Ok(())
        });
    }

    async fn run_logger() {
        #[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
        enum Kind {
            Trace,
            Debug,
            Info,
            Warn,
            Error,
            Exit,
        }

        #[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
        struct Command {
            kind: Kind,
            message: Option<String>,
        }
        let worker = JsWorker::new(include_str!("../bundled/test_logger_worker.js"));

        let trace_succeeded: bool = worker
            .request(Command {
                kind: Kind::Trace,
                message: Some("this is a Trace level log".to_string()),
            })
            .await
            .unwrap();

        let debug_succeeded: bool = worker
            .request(Command {
                kind: Kind::Debug,
                message: Some("this is a Debug level log".to_string()),
            })
            .await
            .unwrap();

        let info_succeeded: bool = worker
            .request(Command {
                kind: Kind::Info,
                message: Some("this is an Info level log".to_string()),
            })
            .await
            .unwrap();

        let warn_succeeded: bool = worker
            .request(Command {
                kind: Kind::Warn,
                message: Some("this is a Warn level log".to_string()),
            })
            .await
            .unwrap();

        let error_succeeded: bool = worker
            .request(Command {
                kind: Kind::Error,
                message: Some("this is an Error level log".to_string()),
            })
            .await
            .unwrap();

        // let's shutdown the js worker before we run assertions,
        // to prevent a potential hang
        let shutdown_succeeded: bool = worker
            .request(Command {
                kind: Kind::Exit,
                message: None,
            })
            .await
            .unwrap();

        assert!(warn_succeeded, "couldn't send warn log command");
        assert!(info_succeeded, "couldn't send info log command");
        assert!(debug_succeeded, "couldn't send debug log command");
        assert!(trace_succeeded, "couldn't send trace log command");
        assert!(error_succeeded, "couldn't send error log command");
        assert!(shutdown_succeeded, "couldn't send shutdown command");
    }

    #[tokio::test]
    // This test ensures crypto.getRandomValues can be called.
    // the uuid dependency relies on it since v9.0
    async fn test_get_random_values() {
        let mut worker = JsWorker::new(include_str!("../bundled/test_get_random_values.js"));

        worker.quit().unwrap();
    }
}
