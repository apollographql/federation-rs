use crate::error::Error;
use async_channel::bounded;
use async_channel::Receiver;
use async_channel::Sender;
use deno_core::{op, Extension, JsRuntime, OpState, RuntimeOptions, Snapshot};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::cell::RefCell;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::rc::Rc;
use std::thread::JoinHandle;

pub(crate) struct JsWorker<Request, Response>
where
    Request: Serialize + Send + Debug + 'static,
    Response: DeserializeOwned + Send + Debug + 'static,
{
    sender: Sender<Request>,
    handle: Option<JoinHandle<()>>,
    receiver: Receiver<Response>,
}

impl<Request, Response> JsWorker<Request, Response>
where
    Request: Serialize + Send + Debug + 'static,
    Response: DeserializeOwned + Send + Debug + 'static,
{
    pub(crate) fn new(worker_source_code: &'static str) -> Self {
        // All channels are bounded(1) so we don't need to use a multiplexer
        let (response_sender, receiver) = bounded::<Response>(1);
        let (sender, request_receiver) = bounded::<Request>(1);

        let handle = std::thread::spawn(move || {
            let my_ext = Extension::builder()
                .ops(vec![send::decl::<Response>(), receive::decl::<Request>()])
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
            receiver,
        }
    }

    pub(crate) async fn request(&self, command: Request) -> Result<Response, Error> {
        self.sender
            .send(command)
            .await
            .map_err(|e| Error::DenoRuntime(format!("couldn't send request {e}")))?;
        self.receiver
            .recv()
            .await
            .map_err(|e| Error::DenoRuntime(format!("request: couldn't receive response {e}")))
    }

    pub(crate) async fn send(&self, command: Request) -> Result<(), Error> {
        self.sender
            .send(command)
            .await
            .map_err(|e| Error::DenoRuntime(format!("send: couldn't send request {e}")))
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

impl<Request, Response> Drop for JsWorker<Request, Response>
where
    Request: Serialize + Send + Debug + 'static,
    Response: DeserializeOwned + Send + Debug + 'static,
{
    fn drop(&mut self) {
        self.quit().unwrap_or_else(|e| eprintln!("{}", e));
    }
}

#[op]
async fn send<Response>(state: Rc<RefCell<OpState>>, payload: Response) -> Result<(), anyhow::Error>
where
    Response: DeserializeOwned + 'static,
{
    let sender = {
        let state = state.borrow();
        // we're cloning here because we don't wanna keep the borrow across an await point
        state.borrow::<Sender<Response>>().clone()
    };

    sender
        .send(payload)
        .await
        .map_err(|e| anyhow::anyhow!("couldn't send response {e}"))
}

#[op]
async fn receive<Request>(state: Rc<RefCell<OpState>>) -> Result<Request, anyhow::Error>
where
    Request: Serialize + Debug + 'static,
{
    let state = state.borrow();
    let receiver = state.borrow::<Receiver<Request>>();
    receiver
        .recv()
        .await
        .map_err(|e| anyhow::anyhow!("op_receive: couldn't send response {e}"))
}
