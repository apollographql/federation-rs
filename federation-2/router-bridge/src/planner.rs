use crate::plan::PlanningErrors;
use crate::worker::JsWorker;
use anyhow::anyhow;
use deno_core::futures::future::BoxFuture;
use deno_core::futures::TryFutureExt;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::oneshot;
use tower::BoxError;
use tower::Service;

#[derive(Clone)]
/// This structure is a query planner service.
pub struct PlannerService<T>
where
    T: DeserializeOwned + Send + Debug + 'static,
{
    planner: Arc<Planner<T>>,
}

impl<T> Debug for PlannerService<T>
where
    T: DeserializeOwned + Send + Debug + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlannerService").finish()
    }
}

impl<T> PlannerService<T>
where
    T: DeserializeOwned + Send + Debug + 'static,
{
    pub async fn new(schema: String) -> Result<Self, anyhow::Error> {
        Ok(Self {
            planner: Arc::new(Planner::new(schema).await?),
        })
    }
}

impl<T> Service<String> for PlannerService<T>
where
    T: DeserializeOwned + Send + Debug + 'static,
{
    type Response = PlanResult<T>;

    type Error = BoxError;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: String) -> Self::Future {
        let planner = self.planner.clone();

        Box::pin(async move {
            planner
                .plan(req, None)
                .map_err(std::convert::Into::into)
                .await
        })
    }
}

// ------------------------------------

type PlanResult<T> = Result<T, PlanningErrors>;

struct Planner<T>
where
    T: DeserializeOwned + Send + Debug + 'static,
{
    worker: Arc<JsWorker<PlanCmd, PlanResult<T>>>,
}

impl<T> Planner<T>
where
    T: DeserializeOwned + Send + Debug + 'static,
{
    async fn new(schema: String) -> Result<Self, anyhow::Error> {
        let worker = JsWorker::new(include_str!("../js-dist/plan_worker.js"));
        worker.send(PlanCmd::UpdateSchema { schema }).await?;

        let worker = Arc::new(worker);

        Ok(Self { worker })
    }

    async fn plan(
        &self,
        query: String,
        operation_name: Option<String>,
    ) -> Result<PlanResult<T>, anyhow::Error> {
        self.worker
            .request(PlanCmd::Plan {
                query,
                operation_name,
            })
            .await
    }
}

impl<T> Drop for Planner<T>
where
    T: DeserializeOwned + Send + Debug + 'static,
{
    fn drop(&mut self) {
        // Send a PlanCmd::Exit signal
        let worker_clone = self.worker.clone();
        let _ = std::thread::spawn(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap();

            let _ = runtime.block_on(async move { worker_clone.send(PlanCmd::Exit).await });
        })
        .join();
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "kind")]
enum PlanCmd {
    UpdateSchema {
        schema: String,
    },
    #[serde(rename_all = "camelCase")]
    Plan {
        query: String,
        operation_name: Option<String>,
    },
    Exit,
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUERY: &str = include_str!("testdata/query.graphql");
    const SCHEMA: &str = include_str!("testdata/schema.graphql");

    #[tokio::test]
    async fn it_works() {
        let planner = Planner::<serde_json::Value>::new(SCHEMA.to_string())
            .await
            .unwrap();

        let plan = planner
            .plan(QUERY.to_string(), None)
            .await
            .unwrap()
            .unwrap();

        insta::assert_snapshot!(serde_json::to_string_pretty(&plan).unwrap());

        let mut service = PlannerService::<serde_json::Value>::new(SCHEMA.to_string())
            .await
            .unwrap();

        let tower_plan = service.call(QUERY.to_string()).await.unwrap().unwrap();

        assert_eq!(plan, tower_plan);
    }
}
