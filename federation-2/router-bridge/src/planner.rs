/*!
 * Instantiate a QueryPlanner from a schema, and perform query planning
*/

use crate::worker::JsWorker;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::sync::Arc;

// ------------------------------------

/// A Deno worker backed query Planner.

pub struct Planner<T>
where
    T: DeserializeOwned + Send + Debug + 'static,
{
    worker: Arc<JsWorker<PlanCmd, T>>,
}

impl<T> Debug for Planner<T>
where
    T: DeserializeOwned + Send + Debug + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Planner").finish()
    }
}

impl<T> Planner<T>
where
    T: DeserializeOwned + Send + Debug + 'static,
{
    /// Instantiate a `Planner` from a schema string
    pub async fn new(schema: String) -> Result<Self, anyhow::Error> {
        let worker = JsWorker::new(include_str!("../js-dist/plan_worker.js"));
        worker.send(PlanCmd::UpdateSchema { schema }).await?;

        let worker = Arc::new(worker);

        Ok(Self { worker })
    }

    /// Plan a query against an instantiated query planner
    pub async fn plan(
        &self,
        query: String,
        operation_name: Option<String>,
    ) -> Result<T, crate::error::Error> {
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

        let plan = planner.plan(QUERY.to_string(), None).await.unwrap();

        insta::assert_snapshot!(serde_json::to_string_pretty(&plan).unwrap());
    }
}
