use rand::Rng;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::sync::atomic::Ordering;
use std::{num::NonZeroUsize, sync::atomic::AtomicUsize};

use std::sync::Arc;
use tokio::task::JoinSet;

use crate::{error::Error, worker::JsWorker};

pub(crate) struct JsWorkerPool {
    workers: Vec<Arc<JsWorker>>,
    pending_requests: Vec<AtomicUsize>,
}

impl JsWorkerPool {
    pub(crate) fn new(worker_source_code: &'static str, size: NonZeroUsize) -> Self {
        let workers: Vec<Arc<JsWorker>> = (0..size.into())
            .map(|_| Arc::new(JsWorker::new(worker_source_code)))
            .collect();

        let pending_requests: Vec<AtomicUsize> =
            (0..size.into()).map(|_| AtomicUsize::new(0)).collect();

        Self {
            workers,
            pending_requests,
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
        let (i, worker) = self.choice_of_two();

        self.pending_requests[i].fetch_add(1, Ordering::SeqCst);
        let result = worker.request(command).await;
        self.pending_requests[i].fetch_add(1, Ordering::SeqCst);

        result
    }

    pub(crate) async fn broadcast_request<Request, Response>(
        &self,
        command: Request,
    ) -> Result<Vec<Response>, Error>
    where
        Request: std::hash::Hash + Serialize + Send + Debug + Clone + 'static,
        Response: DeserializeOwned + Send + Debug + 'static,
    {
        let mut join_set = JoinSet::new();

        #[allow(clippy::unnecessary_to_owned)]
        for worker in self.workers.iter().cloned() {
            let command_clone = command.clone();

            join_set.spawn(async move { worker.request(command_clone).await });
        }

        let mut responses = Vec::new();

        while let Some(result) = join_set.join_next().await {
            let response = result.map_err(|_e| Error::Internal("could not join spawned task".into()))?;
            responses.push(response?);
        }

        Ok(responses)
    }

    pub(crate) async fn broadcast_send<Request>(
        &self,
        id_opt: Option<String>,
        request: Request,
    ) -> Result<(), Error>
    where
        Request: std::hash::Hash + Serialize + Send + Debug + Clone + 'static,
    {
        let mut join_set = JoinSet::new();

        #[allow(clippy::unnecessary_to_owned)]
        for worker in self.workers.iter().cloned() {
            let request_clone = request.clone();
            let id_opt_clone = id_opt.clone();

            join_set.spawn(async move { worker.send(id_opt_clone, request_clone).await });
        }

        let mut results = Vec::new();

        while let Some(result) = join_set.join_next().await {
            let result = result.map_err(|_e| Error::Internal("could not join spawned task".into()))?;
            results.push(result?);
        }

        Ok(())
    }

    fn choice_of_two(&self) -> (usize, &JsWorker) {
        let mut rng = rand::thread_rng();

        let len = self.workers.len();

        let index1 = rng.gen_range(0..len);
        let mut index2 = rng.gen_range(0..len);
        while index2 == index1 {
            index2 = rng.gen_range(0..len);
        }

        let index1_load = &self.pending_requests[index1].load(Ordering::SeqCst);
        let index2_load = &self.pending_requests[index2].load(Ordering::SeqCst);

        let choice = if index1_load < index2_load {
            index1
        } else {
            index2
        };

        (choice, &self.workers[choice])
    }
}
