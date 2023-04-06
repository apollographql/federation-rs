/*!
 * Instantiate a QueryPlanner from a schema, and perform query planning
*/

use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::marker::PhantomData;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

use crate::introspect::IntrospectionResponse;
use crate::worker::JsWorker;

// ------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// Options for the query plan
pub struct QueryPlanOptions {
    /// Use auto fragmentation
    pub auto_fragmentization: bool,
}

/// Default options for query planning
impl Default for QueryPlanOptions {
    /// Default query plan options
    fn default() -> Self {
        Self {
            auto_fragmentization: false,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// This is the context which provides
/// all the information to plan a query against a schema
pub struct OperationalContext {
    /// The graphQL schema
    pub schema: String,
    /// The graphQL query
    pub query: String,
    /// The operation name
    pub operation_name: String,
}

/// An error which occurred during JavaScript planning.
///
/// The shape of this error is meant to mimic that of the error created within
/// JavaScript, which is a [`GraphQLError`] from the [`graphql-js`] library.
///
/// [`graphql-js`]: https://npm.im/graphql
/// [`GraphQLError`]: https://github.com/graphql/graphql-js/blob/3869211/src/error/GraphQLError.js#L18-L75
#[derive(Debug, Error, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct PlanError {
    /// A human-readable description of the error that prevented planning.
    pub message: Option<String>,
    /// [`PlanErrorExtensions`]
    #[serde(deserialize_with = "none_only_if_value_is_null_or_empty_object")]
    pub extensions: Option<PlanErrorExtensions>,
}

/// `none_only_if_value_is_null_or_empty_object`
///
/// This function returns Ok(Some(T)) if a T can be deserialized,
///
/// Ok(None) if data contains Null or an empty object,
/// And fails otherwise, including if the key is missing.
fn none_only_if_value_is_null_or_empty_object<'de, D, T>(data: D) -> Result<Option<T>, D::Error>
where
    D: serde::de::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OptionOrValue<T> {
        Opt(Option<T>),
        Val(serde_json::value::Value),
    }

    let as_option_or_value: Result<OptionOrValue<T>, D::Error> =
        serde::Deserialize::deserialize(data);

    match as_option_or_value {
        Ok(OptionOrValue::Opt(t)) => Ok(t),
        Ok(OptionOrValue::Val(obj)) => {
            if let serde_json::value::Value::Object(o) = &obj {
                if o.is_empty() {
                    return Ok(None);
                }
            }

            Err(serde::de::Error::custom(format!(
                "invalid neither null nor empty object: found {obj:?}"
            )))
        }
        Err(e) => Err(e),
    }
}

impl Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(msg) = &self.message {
            f.write_fmt(format_args!("{code}: {msg}", code = self.code(), msg = msg))
        } else {
            f.write_str(self.code())
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
/// Error codes
pub struct PlanErrorExtensions {
    /// The error code
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The stacktrace if we have one
    pub exception: Option<ExtensionsException>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
/// stacktrace in error extensions
pub struct ExtensionsException {
    /// The stacktrace generated in JavaScript
    pub stacktrace: String,
}

/// An error that was received during planning within JavaScript.
impl PlanError {
    /// Retrieve the error code from an error received during planning.
    pub fn code(&self) -> &str {
        match self.extensions {
            Some(ref ext) => &ext.code,
            None => "UNKNOWN",
        }
    }
}

// ------------------------------------

#[derive(Deserialize, Debug)]
/// The result of a router bridge invocation
pub struct BridgeSetupResult<T> {
    /// The data if setup happened successfully
    pub data: Option<T>,
    /// The errors if the query failed
    pub errors: Option<Vec<PlannerError>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
/// The error location
pub struct Location {
    /// The line number
    pub line: u32,
    /// The column number
    pub column: u32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(untagged)]
/// This contains the set of all errors that can be thrown from deno
pub enum PlannerError {
    /// The deno GraphQLError counterpart
    WorkerGraphQLError(WorkerGraphQLError),
    /// The deno Error counterpart
    WorkerError(WorkerError),
}

impl From<WorkerGraphQLError> for PlannerError {
    fn from(e: WorkerGraphQLError) -> Self {
        Self::WorkerGraphQLError(e)
    }
}

impl From<WorkerError> for PlannerError {
    fn from(e: WorkerError) -> Self {
        Self::WorkerError(e)
    }
}

impl std::fmt::Display for PlannerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkerGraphQLError(graphql_error) => {
                write!(f, "{graphql_error}")
            }
            Self::WorkerError(error) => {
                write!(f, "{error}")
            }
        }
    }
}

/// WorkerError represents the non GraphQLErrors the deno worker can throw.
/// We try to get as much data out of them.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct WorkerError {
    /// The error message
    pub message: Option<String>,
    /// The error kind
    pub name: Option<String>,
    /// A stacktrace if applicable
    pub stack: Option<String>,
    /// [`PlanErrorExtensions`]
    pub extensions: Option<PlanErrorExtensions>,
    /// If an error can be associated to a particular point in the requested
    /// GraphQL document, it should contain a list of locations.
    #[serde(default)]
    pub locations: Vec<Location>,
}

impl std::fmt::Display for WorkerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.message
                .clone()
                .unwrap_or_else(|| "unknown error".to_string())
        )
    }
}

/// WorkerGraphQLError represents the GraphQLErrors the deno worker can throw.
/// We try to get as much data out of them.
/// While they mostly represent GraphQLErrors, they sometimes don't.
/// See [`WorkerError`]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkerGraphQLError {
    /// The error kind
    pub name: String,
    /// A short, human-readable summary of the problem that **SHOULD NOT** change
    /// from occurrence to occurrence of the problem, except for purposes of
    /// localization.
    pub message: String,
    /// If an error can be associated to a particular point in the requested
    /// GraphQL document, it should contain a list of locations.
    #[serde(default)]
    pub locations: Vec<Location>,
    /// [`PlanErrorExtensions`]
    pub extensions: Option<PlanErrorExtensions>,
    /// The original error thrown from a field resolver during execution.
    pub original_error: Option<Box<WorkerError>>,
    /// The reasons why the error was triggered (useful for schema checks)
    #[serde(default)]
    pub causes: Vec<Box<WorkerError>>,
}

impl std::fmt::Display for WorkerGraphQLError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\ncaused by\n{}",
            self.message,
            self.causes
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
/// A list of fields that will be resolved
/// for a given type
pub struct ReferencedFieldsForType {
    /// names of the fields queried
    #[serde(default)]
    pub field_names: Vec<String>,
    /// whether the field is an interface
    #[serde(default)]
    pub is_interface: bool,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
/// UsageReporting fields, that will be used
/// to send stats to uplink/studio
pub struct UsageReporting {
    /// The `stats_report_key` is a unique identifier derived from schema and query.
    /// Metric data  sent to Studio must be aggregated
    /// via grouped key of (`client_name`, `client_version`, `stats_report_key`).
    pub stats_report_key: String,
    /// a list of all types and fields referenced in the query
    #[serde(default)]
    pub referenced_fields_by_type: HashMap<String, ReferencedFieldsForType>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
/// The result of a router bridge plan_worker invocation
pub struct PlanResult<T> {
    /// The data if the query was successfully run
    pub data: Option<T>,
    /// Usage reporting related data such as the
    /// operation signature and referenced fields
    pub usage_reporting: UsageReporting,
    /// The errors if the query failed
    pub errors: Option<Vec<PlanError>>,
}

/// The payload if the plan_worker invocation succeeded
#[derive(Debug)]
pub struct PlanSuccess<T> {
    /// The payload you're looking for
    pub data: T,
    /// Usage reporting related data such as the
    /// operation signature and referenced fields
    pub usage_reporting: UsageReporting,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
/// The result of a router bridge API schema invocation
pub struct ApiSchema {
    /// The data if the query was successfully run
    pub schema: String,
}

/// The payload if the plan_worker invocation failed
#[derive(Debug, Clone)]
pub struct PlanErrors {
    /// The errors the plan_worker invocation failed with
    pub errors: Arc<Vec<PlanError>>,
    /// Usage reporting related data such as the
    /// operation signature and referenced fields
    pub usage_reporting: UsageReporting,
}

impl std::fmt::Display for PlanErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "query validation errors: {}",
            self.errors
                .iter()
                .map(|e| e
                    .message
                    .clone()
                    .unwrap_or_else(|| "UNKNWON ERROR".to_string()))
                .collect::<Vec<String>>()
                .join(", ")
        ))
    }
}

impl<T> PlanResult<T>
where
    T: DeserializeOwned + Send + Debug + 'static,
{
    /// Turn a BridgeResult into an actual Result
    pub fn into_result(self) -> Result<PlanSuccess<T>, PlanErrors> {
        let usage_reporting = self.usage_reporting;
        if let Some(data) = self.data {
            Ok(PlanSuccess {
                data,
                usage_reporting,
            })
        } else {
            let errors = Arc::new(self.errors.unwrap_or_else(|| {
                vec![PlanError {
                    message: Some("an unknown error occured".to_string()),
                    extensions: None,
                }]
            }));
            Err(PlanErrors {
                errors,
                usage_reporting,
            })
        }
    }
}

/// A Deno worker backed query Planner.

pub struct Planner<T>
where
    T: DeserializeOwned + Send + Debug + 'static,
{
    worker: Arc<JsWorker>,
    schema_id: u64,
    t: PhantomData<T>,
}

impl<T> Debug for Planner<T>
where
    T: DeserializeOwned + Send + Debug + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Planner")
            .field("schema_id", &self.schema_id)
            .finish()
    }
}

impl<T> Planner<T>
where
    T: DeserializeOwned + Send + Debug + 'static,
{
    /// Instantiate a `Planner` from a schema string
    pub async fn new(
        schema: String,
        config: QueryPlannerConfig,
    ) -> Result<Self, Vec<PlannerError>> {
        let schema_id: u64 = rand::random();
        let worker = JsWorker::new(include_str!("../bundled/plan_worker.js"));
        let worker_is_set_up = worker
            .request::<PlanCmd, BridgeSetupResult<serde_json::Value>>(PlanCmd::UpdateSchema {
                schema,
                config,
                schema_id,
            })
            .await
            .map_err(|e| {
                vec![WorkerError {
                    name: Some("planner setup error".to_string()),
                    message: Some(e.to_string()),
                    stack: None,
                    extensions: None,
                    locations: Default::default(),
                }
                .into()]
            });

        // Both cases below the mean schema update failed.
        // We need to pay attention here.
        // returning early will drop the worker, which will join the jsruntime thread.
        // however the event loop will run for ever. We need to let the worker know it needs to exit,
        // before we drop the worker
        match worker_is_set_up {
            Err(setup_error) => {
                let _ = worker
                    .request::<PlanCmd, serde_json::Value>(PlanCmd::Exit { schema_id })
                    .await;
                return Err(setup_error);
            }
            Ok(setup) => {
                if let Some(error) = setup.errors {
                    let _ = worker.send(None, PlanCmd::Exit { schema_id }).await;
                    return Err(error);
                }
            }
        }

        let worker = Arc::new(worker);

        Ok(Self {
            worker,
            schema_id,
            t: PhantomData,
        })
    }

    /// Update `Planner` from a schema string
    pub async fn update(
        &self,
        schema: String,
        config: QueryPlannerConfig,
    ) -> Result<Self, Vec<PlannerError>> {
        let schema_id: u64 = rand::random();

        let worker_is_set_up = self
            .worker
            .request::<PlanCmd, BridgeSetupResult<serde_json::Value>>(PlanCmd::UpdateSchema {
                schema,
                config,
                schema_id,
            })
            .await
            .map_err(|e| {
                vec![WorkerError {
                    name: Some("planner setup error".to_string()),
                    message: Some(e.to_string()),
                    stack: None,
                    extensions: None,
                    locations: Default::default(),
                }
                .into()]
            });

        // If the update failed, we keep the existing schema in place
        match worker_is_set_up {
            Err(setup_error) => {
                return Err(setup_error);
            }
            Ok(setup) => {
                if let Some(error) = setup.errors {
                    return Err(error);
                }
            }
        }

        Ok(Self {
            worker: self.worker.clone(),
            schema_id,
            t: PhantomData,
        })
    }

    /// Plan a query against an instantiated query planner
    pub async fn plan(
        &self,
        query: String,
        operation_name: Option<String>,
    ) -> Result<PlanResult<T>, crate::error::Error> {
        self.worker
            .request(PlanCmd::Plan {
                query,
                operation_name,
                schema_id: self.schema_id,
            })
            .await
    }

    /// Generate the API schema from the current schema
    pub async fn api_schema(&self) -> Result<ApiSchema, crate::error::Error> {
        self.worker
            .request(PlanCmd::ApiSchema {
                schema_id: self.schema_id,
            })
            .await
    }

    /// Generate the introspection response for this query
    pub async fn introspect(
        &self,
        query: String,
    ) -> Result<IntrospectionResponse, crate::error::Error> {
        self.worker
            .request(PlanCmd::Introspect {
                query,
                schema_id: self.schema_id,
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
        let schema_id = self.schema_id;
        let _ = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap();

            let _ = runtime.block_on(async move {
                worker_clone.send(None, PlanCmd::Exit { schema_id }).await
            });
        })
        .join();
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(tag = "kind")]
enum PlanCmd {
    #[serde(rename_all = "camelCase")]
    UpdateSchema {
        schema: String,
        config: QueryPlannerConfig,
        schema_id: u64,
    },
    #[serde(rename_all = "camelCase")]
    Plan {
        query: String,
        operation_name: Option<String>,
        schema_id: u64,
    },
    #[serde(rename_all = "camelCase")]
    ApiSchema { schema_id: u64 },
    #[serde(rename_all = "camelCase")]
    Introspect { query: String, schema_id: u64 },
    #[serde(rename_all = "camelCase")]
    Exit { schema_id: u64 },
}
#[derive(Serialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
/// Query planner configuration
pub struct QueryPlannerConfig {
    //exposeDocumentNodeInFetchNode?: boolean;

    // Side-note: implemented as an object instead of single boolean because we expect to add more to this soon
    // enough. In particular, once defer-passthrough to subgraphs is implemented, the idea would be to add a
    // new `passthroughSubgraphs` option that is the list of subgraph to which we can pass-through some @defer
    // (and it would be empty by default). Similarly, once we support @stream, grouping the options here will
    // make sense too.
    /// Option for `@defer` directive support
    pub incremental_delivery: Option<IncrementalDeliverySupport>,
}

impl Default for QueryPlannerConfig {
    fn default() -> Self {
        Self {
            incremental_delivery: Some(IncrementalDeliverySupport {
                enable_defer: Some(false),
            }),
        }
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
/// Option for `@defer` directive support
pub struct IncrementalDeliverySupport {
    /// Enables @defer support by the query planner.
    ///
    /// If set, then the query plan for queries having some @defer will contains some `DeferNode` (see `QueryPlan.ts`).
    ///
    /// Defaults to false (meaning that the @defer are ignored).
    #[serde(default)]
    pub enable_defer: Option<bool>,
}

#[cfg(test)]
mod tests {
    use futures::stream::StreamExt;
    use futures::stream::{self};

    use super::*;

    const QUERY: &str = include_str!("testdata/query.graphql");
    const QUERY2: &str = include_str!("testdata/query2.graphql");
    const MULTIPLE_QUERIES: &str = include_str!("testdata/query_with_multiple_operations.graphql");
    const NO_OPERATION: &str = include_str!("testdata/no_operation.graphql");

    const MULTIPLE_ANONYMOUS_QUERIES: &str =
        include_str!("testdata/query_with_multiple_anonymous_operations.graphql");
    const NAMED_QUERY: &str = include_str!("testdata/named_query.graphql");
    const SCHEMA: &str = include_str!("testdata/schema.graphql");
    const SCHEMA_WITHOUT_REVIEW_BODY: &str =
        include_str!("testdata/schema_without_review_body.graphql");
    const CORE_IN_V0_1: &str = include_str!("testdata/core_in_v0.1.graphql");
    const UNSUPPORTED_FEATURE: &str = include_str!("testdata/unsupported_feature.graphql");
    const UNSUPPORTED_FEATURE_FOR_EXECUTION: &str =
        include_str!("testdata/unsupported_feature_for_execution.graphql");
    const UNSUPPORTED_FEATURE_FOR_SECURITY: &str =
        include_str!("testdata/unsupported_feature_for_security.graphql");

    #[tokio::test]
    async fn anonymous_query_works() {
        let planner =
            Planner::<serde_json::Value>::new(SCHEMA.to_string(), QueryPlannerConfig::default())
                .await
                .unwrap();

        let payload = planner
            .plan(QUERY.to_string(), None)
            .await
            .unwrap()
            .into_result()
            .unwrap();
        insta::assert_snapshot!(serde_json::to_string_pretty(&payload.data).unwrap());
        insta::with_settings!({sort_maps => true}, {
            insta::assert_json_snapshot!(payload.usage_reporting);
        });
    }

    #[tokio::test]
    async fn named_query_works() {
        let planner =
            Planner::<serde_json::Value>::new(SCHEMA.to_string(), QueryPlannerConfig::default())
                .await
                .unwrap();

        let payload = planner
            .plan(NAMED_QUERY.to_string(), None)
            .await
            .unwrap()
            .into_result()
            .unwrap();
        insta::assert_snapshot!(serde_json::to_string_pretty(&payload.data).unwrap());
        insta::with_settings!({sort_maps => true}, {
            insta::assert_json_snapshot!(payload.usage_reporting);
        });
    }

    #[tokio::test]
    async fn named_query_with_several_choices_works() {
        let planner =
            Planner::<serde_json::Value>::new(SCHEMA.to_string(), QueryPlannerConfig::default())
                .await
                .unwrap();

        let payload = planner
            .plan(
                MULTIPLE_QUERIES.to_string(),
                Some("MyFirstName".to_string()),
            )
            .await
            .unwrap()
            .into_result()
            .unwrap();
        insta::assert_snapshot!(serde_json::to_string_pretty(&payload.data).unwrap());
        insta::with_settings!({sort_maps => true}, {
            insta::assert_json_snapshot!(payload.usage_reporting);
        });
    }

    #[tokio::test]
    async fn named_query_with_operation_name_works() {
        let planner =
            Planner::<serde_json::Value>::new(SCHEMA.to_string(), QueryPlannerConfig::default())
                .await
                .unwrap();

        let payload = planner
            .plan(
                NAMED_QUERY.to_string(),
                Some("MyFirstAndLastName".to_string()),
            )
            .await
            .unwrap()
            .into_result()
            .unwrap();
        insta::assert_snapshot!(serde_json::to_string_pretty(&payload.data).unwrap());
        insta::with_settings!({sort_maps => true}, {
            insta::assert_json_snapshot!(payload.usage_reporting);
        });
    }

    #[tokio::test]
    async fn parse_errors_return_the_right_usage_reporting_data() {
        let planner =
            Planner::<serde_json::Value>::new(SCHEMA.to_string(), QueryPlannerConfig::default())
                .await
                .unwrap();

        let payload = planner
            .plan("this query will definitely not parse".to_string(), None)
            .await
            .unwrap()
            .into_result()
            .unwrap_err();

        assert_eq!(
            "Syntax Error: Unexpected Name \"this\".",
            payload.errors[0].message.as_ref().unwrap()
        );
        assert_eq!(
            "## GraphQLParseFailure\n",
            payload.usage_reporting.stats_report_key
        );
    }

    #[tokio::test]
    async fn validation_errors_return_the_right_usage_reporting_data() {
        let planner =
            Planner::<serde_json::Value>::new(SCHEMA.to_string(), QueryPlannerConfig::default())
                .await
                .unwrap();

        let payload = planner
            .plan(
                // These two fragments will spread themselves into a cycle, which is invalid per NoFragmentCyclesRule.
                "\
            fragment thatUserFragment1 on User {
                id
                ...thatUserFragment2
            }
            fragment thatUserFragment2 on User {
                id
                ...thatUserFragment1
            }
            query { me { id ...thatUserFragment1 } }"
                    .to_string(),
                None,
            )
            .await
            .unwrap()
            .into_result()
            .unwrap_err();

        assert_eq!(
            "Cannot spread fragment \"thatUserFragment1\" within itself via \"thatUserFragment2\".",
            payload.errors[0].message.as_ref().unwrap()
        );
        assert_eq!(
            "## GraphQLValidationFailure\n",
            payload.usage_reporting.stats_report_key
        );
    }

    #[tokio::test]
    async fn unknown_operation_name_errors_return_the_right_usage_reporting_data() {
        let planner =
            Planner::<serde_json::Value>::new(SCHEMA.to_string(), QueryPlannerConfig::default())
                .await
                .unwrap();

        let payload = planner
            .plan(
                QUERY.to_string(),
                Some("ThisOperationNameDoesntExist".to_string()),
            )
            .await
            .unwrap()
            .into_result()
            .unwrap_err();

        assert_eq!(
            "Unknown operation named \"ThisOperationNameDoesntExist\"",
            payload.errors[0].message.as_ref().unwrap()
        );
        assert_eq!(
            "## GraphQLUnknownOperationName\n",
            payload.usage_reporting.stats_report_key
        );
    }

    #[tokio::test]
    async fn must_provide_operation_name_errors_return_the_right_usage_reporting_data() {
        let planner =
            Planner::<serde_json::Value>::new(SCHEMA.to_string(), QueryPlannerConfig::default())
                .await
                .unwrap();

        let payload = planner
            .plan(MULTIPLE_QUERIES.to_string(), None)
            .await
            .unwrap()
            .into_result()
            .unwrap_err();

        assert_eq!(
            "Must provide operation name if query contains multiple operations.",
            payload.errors[0].message.as_ref().unwrap()
        );
        assert_eq!(
            "## GraphQLUnknownOperationName\n",
            payload.usage_reporting.stats_report_key
        );
    }

    #[tokio::test]
    async fn multiple_anonymous_queries_return_the_expected_usage_reporting_data() {
        let planner =
            Planner::<serde_json::Value>::new(SCHEMA.to_string(), QueryPlannerConfig::default())
                .await
                .unwrap();

        let payload = planner
            .plan(MULTIPLE_ANONYMOUS_QUERIES.to_string(), None)
            .await
            .unwrap()
            .into_result()
            .unwrap_err();

        assert_eq!(
            "This anonymous operation must be the only defined operation.",
            payload.errors[0].message.as_ref().unwrap()
        );
        assert_eq!(
            "## GraphQLValidationFailure\n",
            payload.usage_reporting.stats_report_key
        );
    }

    #[tokio::test]
    async fn no_operation_in_document() {
        let planner =
            Planner::<serde_json::Value>::new(SCHEMA.to_string(), QueryPlannerConfig::default())
                .await
                .unwrap();

        let payload = planner
            .plan(NO_OPERATION.to_string(), None)
            .await
            .unwrap()
            .into_result()
            .unwrap_err();

        assert_eq!(
            "Fragment \"thatUserFragment1\" is never used.",
            payload.errors[0].message.as_ref().unwrap()
        );
        assert_eq!(
            "## GraphQLValidationFailure\n",
            payload.usage_reporting.stats_report_key
        );
    }

    #[tokio::test]
    // A series of queries that should fail graphql-js's validate function.  The federation
    // query planning logic automatically does some validation in order to do its duties.
    // Some, but not all, of that validation is also handled by the graphql-js validator.
    // However, we are trying to assert that we are testing graphql-js validation, not
    // Federation's query planner validation.  So we run a few validations which we do not
    // expect to every show up in Federation's query planner validation.
    // This one is for the NoFragmentCyclesRule in graphql/validate
    async fn invalid_graphql_validation_1_is_caught() {
        let errors= vec![PlanError {
                message: Some("Cannot spread fragment \"thatUserFragment1\" within itself via \"thatUserFragment2\".".to_string()),
                extensions: Some(PlanErrorExtensions {
                    code: String::from("GRAPHQL_VALIDATION_FAILED"),
                    exception: None,
                }),
            }];

        assert_errors(
            errors,
            // These two fragments will spread themselves into a cycle, which is invalid per NoFragmentCyclesRule.
            "\
        fragment thatUserFragment1 on User {
            id
            ...thatUserFragment2
        }
        fragment thatUserFragment2 on User {
            id
            ...thatUserFragment1
        }
        query { me { id ...thatUserFragment1 } }"
                .to_string(),
            None,
        )
        .await;
    }

    #[tokio::test]
    // A series of queries that should fail graphql-js's validate function.  The federation
    // query planning logic automatically does some validation in order to do its duties.
    // Some, but not all, of that validation is also handled by the graphql-js validator.
    // However, we are trying to assert that we are testing graphql-js validation, not
    // Federation's query planner validation.  So we run a few validations which we do not
    // expect to every show up in Federation's query planner validation.
    // This one is for the ScalarLeafsRule in graphql/validate
    async fn invalid_graphql_validation_2_is_caught() {
        let errors = vec![PlanError {
            message: Some(
                "Field \"id\" must not have a selection since type \"ID!\" has no subfields."
                    .to_string(),
            ),
            extensions: Some(PlanErrorExtensions {
                code: String::from("GRAPHQL_VALIDATION_FAILED"),
                exception: None,
            }),
        }];

        assert_errors(
            errors,
            // This Book resolver requires a selection set, per the schema.
            "{ me { id { absolutelyNotAcceptableLeaf } } }".to_string(),
            None,
        )
        .await;
    }

    #[tokio::test]
    // A series of queries that should fail graphql-js's validate function.  The federation
    // query planning logic automatically does some validation in order to do its duties.
    // Some, but not all, of that validation is also handled by the graphql-js validator.
    // However, we are trying to assert that we are testing graphql-js validation, not
    // Federation's query planner validation.  So we run a few validations which we do not
    // expect to every show up in Federation's query planner validation.
    // This one is for NoUnusedFragmentsRule in graphql/validate
    async fn invalid_graphql_validation_3_is_caught() {
        let errors = vec![PlanError {
            message: Some("Fragment \"UnusedTestFragment\" is never used.".to_string()),
            extensions: Some(PlanErrorExtensions {
                code: String::from("GRAPHQL_VALIDATION_FAILED"),
                exception: None,
            }),
        }];

        assert_errors(
            errors,
            // This Book resolver requires a selection set, per the schema.
            "fragment UnusedTestFragment on User { id } query { me { id } }".to_string(),
            None,
        )
        .await;
    }

    #[tokio::test]
    async fn invalid_federation_validation_is_caught() {
        let errors = vec![PlanError {
            message: Some(
                "Must provide operation name if query contains multiple operations.".to_string(),
            ),
            extensions: Some(PlanErrorExtensions {
                code: "GRAPHQL_VALIDATION_FAILED".to_string(),
                exception: None,
            }),
        }];

        assert_errors(
            errors, // This requires passing an operation name (because there are multiple operations)
            // but we have not done that! Therefore, we expect a validation error from planning.
            "query Operation1 { me { id } } query Operation2 { me { id } }".to_string(),
            None,
        )
        .await;
    }

    #[tokio::test]
    async fn invalid_schema_is_caught() {
        let expected_errors: Vec<PlannerError> = vec![WorkerGraphQLError {
            name: "GraphQLError".to_string(),
            message: "Syntax Error: Unexpected Name \"Garbage\".".to_string(),
            extensions: None,
            locations: vec![Location { line: 1, column: 1 }],
            original_error: None,
            causes: vec![],
        }
        .into()];

        let actual_error =
            Planner::<serde_json::Value>::new("Garbage".to_string(), QueryPlannerConfig::default())
                .await
                .unwrap_err();

        assert_eq!(expected_errors, actual_error);
    }

    #[tokio::test]
    async fn syntactically_incorrect_query_is_caught() {
        let errors = vec![PlanError {
            message: Some("Syntax Error: Unexpected Name \"Garbage\".".to_string()),
            extensions: Some(PlanErrorExtensions {
                code: String::from("GRAPHQL_PARSE_FAILED"),
                exception: None,
            }),
        }];

        assert_errors(errors, "Garbage".to_string(), None).await;
    }

    #[tokio::test]
    async fn query_missing_subfields() {
        let expected_error_message = r#"Field "reviews" of type "[Review]" must have a selection of subfields. Did you mean "reviews { ... }"?"#;

        let errors = vec![PlanError {
            message: Some(expected_error_message.to_string()),
            extensions: Some(PlanErrorExtensions {
                code: String::from("GRAPHQL_VALIDATION_FAILED"),
                exception: None,
            }),
        }];

        assert_errors(
            errors,
            // This query contains reviews, which requires subfields
            "query ExampleQuery { me { id reviews } }".to_string(),
            None,
        )
        .await;
    }

    #[tokio::test]
    async fn query_field_that_doesnt_exist() {
        let expected_error_message = r#"Cannot query field "thisDoesntExist" on type "Query"."#;
        let errors = vec![PlanError {
            message: Some(expected_error_message.to_string()),
            extensions: Some(PlanErrorExtensions {
                code: String::from("GRAPHQL_VALIDATION_FAILED"),
                exception: None,
            }),
        }];

        assert_errors(
            errors,
            // This query contains reviews, which requires subfields
            "query ExampleQuery { thisDoesntExist }".to_string(),
            None,
        )
        .await;
    }

    async fn assert_errors(
        expected_errors: Vec<PlanError>,
        query: String,
        operation_name: Option<String>,
    ) {
        let planner =
            Planner::<serde_json::Value>::new(SCHEMA.to_string(), QueryPlannerConfig::default())
                .await
                .unwrap();

        let actual = planner.plan(query, operation_name).await.unwrap();

        assert_eq!(expected_errors, actual.errors.unwrap());
    }

    #[tokio::test]
    async fn it_doesnt_race() {
        let planner =
            Planner::<serde_json::Value>::new(SCHEMA.to_string(), QueryPlannerConfig::default())
                .await
                .unwrap();

        let query_1_response = planner
            .plan(QUERY.to_string(), None)
            .await
            .unwrap()
            .data
            .unwrap();

        let query_2_response = planner
            .plan(QUERY2.to_string(), None)
            .await
            .unwrap()
            .data
            .unwrap();

        let all_futures = stream::iter((0..1000).map(|i| {
            let (query, fut) = if i % 2 == 0 {
                (QUERY, planner.plan(QUERY.to_string(), None))
            } else {
                (QUERY2, planner.plan(QUERY2.to_string(), None))
            };

            async move { (query, fut.await.unwrap()) }
        }));

        all_futures
            .for_each_concurrent(None, |fut| async {
                let (query, plan_result) = fut.await;
                if query == QUERY {
                    assert_eq!(query_1_response, plan_result.data.unwrap());
                } else {
                    assert_eq!(query_2_response, plan_result.data.unwrap());
                }
            })
            .await;
    }

    #[tokio::test]
    async fn error_on_core_in_v0_1() {
        let expected_errors: Vec<PlannerError> = vec![
            WorkerGraphQLError {
                name: "GraphQLError".to_string(),
                message: r#"one or more checks failed. Caused by:
the `for:` argument is unsupported by version v0.1 of the core spec. Please upgrade to at least @core v0.2 (https://specs.apollo.dev/core/v0.2).

GraphQL request:2:1
1 | schema
2 | @core(feature: "https://specs.apollo.dev/core/v0.1")
  | ^
3 | @core(feature: "https://specs.apollo.dev/join/v0.1", for: EXECUTION)

GraphQL request:3:1
2 | @core(feature: "https://specs.apollo.dev/core/v0.1")
3 | @core(feature: "https://specs.apollo.dev/join/v0.1", for: EXECUTION)
  | ^
4 | @core(

GraphQL request:4:1
3 | @core(feature: "https://specs.apollo.dev/join/v0.1", for: EXECUTION)
4 | @core(
  | ^
5 |     feature: "https://specs.apollo.dev/something-unsupported/v0.1"

feature https://specs.apollo.dev/something-unsupported/v0.1 is for: SECURITY but is unsupported

GraphQL request:4:1
3 | @core(feature: "https://specs.apollo.dev/join/v0.1", for: EXECUTION)
4 | @core(
  | ^
5 |     feature: "https://specs.apollo.dev/something-unsupported/v0.1""#.to_string(),
                locations: Default::default(),
                extensions: Some(PlanErrorExtensions {
                    code: "CheckFailed".to_string(),
                    exception: None
                }),
                original_error: None,
                causes: vec![
                    Box::new(WorkerError {
                        message: Some("the `for:` argument is unsupported by version v0.1 of the core spec. Please upgrade to at least @core v0.2 (https://specs.apollo.dev/core/v0.2).".to_string()),
                        name: None,
                        stack: None,
                        extensions: Some(PlanErrorExtensions { code: "UNSUPPORTED_LINKED_FEATURE".to_string(), exception: None }),
                        locations: vec![Location { line: 2, column: 1 }, Location { line: 3, column: 1 }, Location { line: 4, column: 1 }]
                    }),
                    Box::new(WorkerError {
                        message: Some("feature https://specs.apollo.dev/something-unsupported/v0.1 is for: SECURITY but is unsupported".to_string()),
                        name: None,
                        stack: None,
                        extensions: Some(PlanErrorExtensions { code: "UNSUPPORTED_LINKED_FEATURE".to_string(), exception: None }),
                        locations: vec![Location { line: 4, column: 1 }]
                    })
                ],
            }.into()
        ];
        let actual_errors = Planner::<serde_json::Value>::new(
            CORE_IN_V0_1.to_string(),
            QueryPlannerConfig::default(),
        )
        .await
        .unwrap_err();

        pretty_assertions::assert_eq!(expected_errors, actual_errors);
    }

    #[tokio::test]
    async fn unsupported_feature_without_for() {
        // this should not return an error
        // see gateway test "it doesn't throw errors when using unsupported features which have no `for:` argument"
        Planner::<serde_json::Value>::new(
            UNSUPPORTED_FEATURE.to_string(),
            QueryPlannerConfig::default(),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn unsupported_feature_for_execution() {
        let expected_errors: Vec<PlannerError> = vec![
            WorkerGraphQLError {
                name: "GraphQLError".to_string(),
                message: r#"one or more checks failed. Caused by:
feature https://specs.apollo.dev/unsupported-feature/v0.1 is for: EXECUTION but is unsupported

GraphQL request:4:9
3 |         @core(feature: "https://specs.apollo.dev/join/v0.1", for: EXECUTION)
4 |         @core(
  |         ^
5 |           feature: "https://specs.apollo.dev/unsupported-feature/v0.1""#.to_string(),
                locations: Default::default(),
                extensions: Some(PlanErrorExtensions {
                    code: "CheckFailed".to_string(),
                    exception: None
                }),
                original_error: None,
                causes: vec![
                    Box::new(WorkerError {
                        message: Some("feature https://specs.apollo.dev/unsupported-feature/v0.1 is for: EXECUTION but is unsupported".to_string()),
                        name: None,
                        stack: None,
                        extensions: Some(PlanErrorExtensions { code: "UNSUPPORTED_LINKED_FEATURE".to_string(), exception: None }),
                        locations: vec![Location { line: 4, column: 9 }]
                    }),
                ],
            }.into()
        ];
        let actual_errors = Planner::<serde_json::Value>::new(
            UNSUPPORTED_FEATURE_FOR_EXECUTION.to_string(),
            QueryPlannerConfig::default(),
        )
        .await
        .unwrap_err();

        pretty_assertions::assert_eq!(expected_errors, actual_errors);
    }

    #[tokio::test]
    async fn unsupported_feature_for_security() {
        let expected_errors: Vec<PlannerError> = vec![WorkerGraphQLError {
            name:"GraphQLError".into(),
            message: r#"one or more checks failed. Caused by:
feature https://specs.apollo.dev/unsupported-feature/v0.1 is for: SECURITY but is unsupported

GraphQL request:4:9
3 |         @core(feature: "https://specs.apollo.dev/join/v0.1", for: EXECUTION)
4 |         @core(
  |         ^
5 |           feature: "https://specs.apollo.dev/unsupported-feature/v0.1""#.to_string(),
            locations: vec![],
            extensions: Some(PlanErrorExtensions {
                code: "CheckFailed".to_string(),
                exception: None
            }),
            original_error: None,
            causes: vec![Box::new(WorkerError {
                message: Some("feature https://specs.apollo.dev/unsupported-feature/v0.1 is for: SECURITY but is unsupported".to_string()),
                extensions: Some(PlanErrorExtensions {
                    code: "UNSUPPORTED_LINKED_FEATURE".to_string(),
                    exception: None
                }),
                name: None,
                stack: None,
                locations: vec![Location { line: 4, column: 9 }]
            })],
        }
        .into()];
        let actual_errors = Planner::<serde_json::Value>::new(
            UNSUPPORTED_FEATURE_FOR_SECURITY.to_string(),
            QueryPlannerConfig::default(),
        )
        .await
        .unwrap_err();

        pretty_assertions::assert_eq!(expected_errors, actual_errors);
    }

    #[tokio::test]
    async fn api_schema() {
        let planner =
            Planner::<serde_json::Value>::new(SCHEMA.to_string(), QueryPlannerConfig::default())
                .await
                .unwrap();

        let api_schema = planner.api_schema().await.unwrap();
        insta::assert_snapshot!(api_schema.schema);
    }
    // This string is the result of calling getIntrospectionQuery() from the 'graphql' js package.
    static INTROSPECTION: &str = r#"
query IntrospectionQuery {
__schema {
    queryType {
        name
    }
    mutationType {
        name
    }
    subscriptionType {
        name
    }
    types {
        ...FullType
    }
    directives {
        name
        description
        locations
        args {
            ...InputValue
        }
    }
}
}

fragment FullType on __Type {
kind
name
description

fields(includeDeprecated: true) {
    name
    description
    args {
        ...InputValue
    }
    type {
        ...TypeRef
    }
    isDeprecated
    deprecationReason
}
inputFields {
    ...InputValue
}
interfaces {
    ...TypeRef
}
enumValues(includeDeprecated: true) {
    name
    description
    isDeprecated
    deprecationReason
}
possibleTypes {
    ...TypeRef
}
}

fragment InputValue on __InputValue {
name
description
type {
    ...TypeRef
}
defaultValue
}

fragment TypeRef on __Type {
kind
name
ofType {
    kind
    name
    ofType {
        kind
        name
        ofType {
            kind
            name
                ofType {
                kind
                name
                ofType {
                    kind
                    name
                        ofType {
                        kind
                        name
                        ofType {
                            kind
                            name
                        }
                    }
                }
            }
        }
    }
}
}
"#;
    #[tokio::test]
    async fn introspect() {
        let planner =
            Planner::<serde_json::Value>::new(SCHEMA.to_string(), QueryPlannerConfig::default())
                .await
                .unwrap();

        let introspection_response = planner.introspect(INTROSPECTION.to_string()).await.unwrap();
        insta::assert_json_snapshot!(serde_json::to_value(introspection_response).unwrap());
    }

    #[tokio::test]
    async fn planner_update() {
        let query = "{ me { id name {first } reviews { id author { name { first } } body } } }";
        let planner = Planner::<serde_json::Value>::new(
            SCHEMA_WITHOUT_REVIEW_BODY.to_string(),
            QueryPlannerConfig::default(),
        )
        .await
        .unwrap();
        let query_plan1 = planner
            .plan(query.to_string(), None)
            .await
            .unwrap()
            .into_result()
            .unwrap_err();
        insta::assert_snapshot!(&format!("{query_plan1:#?}"));
        let api_schema1 = planner.api_schema().await.unwrap();
        insta::assert_snapshot!(api_schema1.schema);
        let introspected_schema1 = planner.introspect(INTROSPECTION.to_string()).await.unwrap();

        let updated_planner = planner
            .update(SCHEMA.to_string(), QueryPlannerConfig::default())
            .await
            .unwrap();
        let query_plan2 = updated_planner
            .plan(query.to_string(), None)
            .await
            .unwrap()
            .into_result()
            .unwrap();
        insta::assert_snapshot!(serde_json::to_string_pretty(&query_plan2.data).unwrap());
        let api_schema2 = updated_planner.api_schema().await.unwrap();
        insta::assert_snapshot!(api_schema2.schema);

        // we should still be able to call the old planner, and it must have kept the same schema
        assert_eq!(
            planner.introspect(INTROSPECTION.to_string()).await.unwrap(),
            introspected_schema1
        );

        let introspected_schema2 = updated_planner
            .introspect(INTROSPECTION.to_string())
            .await
            .unwrap();
        assert_ne!(introspected_schema1, introspected_schema2);

        // Now we drop the old planner. The updated planner should still work
        drop(planner);

        assert_eq!(
            query_plan2.data,
            updated_planner
                .plan(query.to_string(), None)
                .await
                .unwrap()
                .into_result()
                .unwrap()
                .data
        );
    }
}

#[cfg(test)]
mod planning_error {
    use std::collections::HashMap;

    use crate::planner::PlanError;
    use crate::planner::PlanErrorExtensions;
    use crate::planner::ReferencedFieldsForType;
    use crate::planner::UsageReporting;

    #[test]
    #[should_panic(
        expected = "Result::unwrap()` on an `Err` value: Error(\"missing field `extensions`\", line: 1, column: 2)"
    )]
    fn deserialize_empty_planning_error() {
        let raw = "{}";
        serde_json::from_str::<PlanError>(raw).unwrap();
    }

    #[test]
    #[should_panic(
        expected = "Result::unwrap()` on an `Err` value: Error(\"missing field `extensions`\", line: 1, column: 44)"
    )]
    fn deserialize_planning_error_missing_extension() {
        let raw = r#"{ "message": "something terrible happened" }"#;
        serde_json::from_str::<PlanError>(raw).unwrap();
    }

    #[test]
    fn deserialize_planning_error_with_extension() {
        let raw = r#"{
            "message": "something terrible happened",
            "extensions": {
                "code": "E_TEST_CASE"
            }
        }"#;

        let expected = PlanError {
            message: Some("something terrible happened".to_string()),
            extensions: Some(PlanErrorExtensions {
                code: "E_TEST_CASE".to_string(),
                exception: None,
            }),
        };

        assert_eq!(expected, serde_json::from_str(raw).unwrap());
    }

    #[test]
    fn deserialize_planning_error_with_empty_object_extension() {
        let raw = r#"{
            "extensions": {}
        }"#;
        let expected = PlanError {
            message: None,
            extensions: None,
        };

        assert_eq!(expected, serde_json::from_str(raw).unwrap());
    }

    #[test]
    fn deserialize_planning_error_with_null_extension() {
        let raw = r#"{
            "extensions": null
        }"#;
        let expected = PlanError {
            message: None,
            extensions: None,
        };

        assert_eq!(expected, serde_json::from_str(raw).unwrap());
    }

    #[test]
    fn deserialize_referenced_fields_for_type_defaults() {
        let raw = r#"{}"#;
        let expected = ReferencedFieldsForType {
            field_names: Vec::new(),
            is_interface: false,
        };

        assert_eq!(expected, serde_json::from_str(raw).unwrap());
    }

    #[test]
    fn deserialize_usage_reporting_with_defaults() {
        let raw = r#"{
            "statsReportKey": "thisIsAtest"
        }"#;
        let expected = UsageReporting {
            stats_report_key: "thisIsAtest".to_string(),
            referenced_fields_by_type: HashMap::new(),
        };

        assert_eq!(expected, serde_json::from_str(raw).unwrap());
    }
}

#[cfg(test)]
mod error_display {
    use super::*;

    #[test]
    fn error_on_core_in_v0_1_display() {
        let expected = r#"one or more checks failed
caused by
the `for:` argument is unsupported by version v0.1 of the core spec. Please upgrade to at least @core v0.2 (https://specs.apollo.dev/core/v0.2).
feature https://specs.apollo.dev/something-unsupported/v0.1 is for: SECURITY but is unsupported"#;

        let error_to_display: PlannerError = WorkerGraphQLError {
            name: "CheckFailed".to_string(),
            message: "one or more checks failed".to_string(),
            locations: Default::default(),
            extensions: Some(PlanErrorExtensions {
                code: "CheckFailed".to_string(),
                exception: None
            }),
            original_error: None,
            causes: vec![
                Box::new(WorkerError {
                    message: Some("the `for:` argument is unsupported by version v0.1 of the core spec. Please upgrade to at least @core v0.2 (https://specs.apollo.dev/core/v0.2).".to_string()),
                    name: None,
                    stack: None,
                    extensions: Some(PlanErrorExtensions { code: "ForUnsupported".to_string(), exception: None }),
                    locations: vec![Location { line: 2, column: 1 }, Location { line: 3, column: 1 }, Location { line: 4, column: 1 }]
                }),
                Box::new(WorkerError {
                    message: Some("feature https://specs.apollo.dev/something-unsupported/v0.1 is for: SECURITY but is unsupported".to_string()),
                    name: None,
                    stack: None,
                    extensions: Some(PlanErrorExtensions { code: "UnsupportedFeature".to_string(), exception: None }),
                    locations: vec![Location { line: 4, column: 1 }]
                })
            ],
        }.into();

        assert_eq!(expected.to_string(), error_to_display.to_string());
    }

    #[test]
    fn unsupported_feature_for_execution_display() {
        let expected = r#"one or more checks failed
caused by
feature https://specs.apollo.dev/unsupported-feature/v0.1 is for: EXECUTION but is unsupported"#;

        let error_to_display: PlannerError = WorkerGraphQLError {
            name: "CheckFailed".to_string(),
            message: "one or more checks failed".to_string(),
            locations: Default::default(),
            extensions: Some(PlanErrorExtensions {
                code: "CheckFailed".to_string(),
                exception: None
            }),
            original_error: None,
            causes: vec![
                Box::new(WorkerError {
                    message: Some("feature https://specs.apollo.dev/unsupported-feature/v0.1 is for: EXECUTION but is unsupported".to_string()),
                    name: None,
                    stack: None,
                    extensions: Some(PlanErrorExtensions { code: "UnsupportedFeature".to_string(), exception: None }),
                    locations: vec![Location { line: 4, column: 9 }]
                }),
            ],
        }.into();

        assert_eq!(expected.to_string(), error_to_display.to_string());
    }

    #[test]
    fn unsupported_feature_for_security_display() {
        let expected = r#"one or more checks failed
caused by
feature https://specs.apollo.dev/unsupported-feature/v0.1 is for: SECURITY but is unsupported"#;

        let error_to_display: PlannerError = WorkerGraphQLError {
            name: "CheckFailed".into(),
            message: "one or more checks failed".to_string(),
            locations: vec![],
            extensions: Some(PlanErrorExtensions {
                code: "CheckFailed".to_string(),
                exception: None
            }),
            original_error: None,
            causes: vec![Box::new(WorkerError {
                message: Some("feature https://specs.apollo.dev/unsupported-feature/v0.1 is for: SECURITY but is unsupported".to_string()),
                extensions: Some(PlanErrorExtensions {
                    code: "UnsupportedFeature".to_string(),
                    exception: None
                }),
                name: None,
                stack: None,
                locations: vec![Location { line: 4, column: 9 }]
            })],
        }
        .into();

        assert_eq!(expected.to_string(), error_to_display.to_string());
    }

    #[tokio::test]
    async fn defer_with_fragment() {
        let schema = r#"
        schema
          @link(url: "https://specs.apollo.dev/link/v1.0")
          @link(url: "https://specs.apollo.dev/join/v0.2", for: EXECUTION)
        {
          query: Query
        }
        
        directive @join__field(graph: join__Graph!, requires: join__FieldSet, provides: join__FieldSet, type: String, external: Boolean, override: String, usedOverridden: Boolean) repeatable on FIELD_DEFINITION | INPUT_FIELD_DEFINITION
        directive @join__graph(name: String!, url: String!) on ENUM_VALUE
        directive @join__implements(graph: join__Graph!, interface: String!) repeatable on OBJECT | INTERFACE
        directive @join__type(graph: join__Graph!, key: join__FieldSet, extension: Boolean! = false, resolvable: Boolean! = true) repeatable on OBJECT | INTERFACE | UNION | ENUM | INPUT_OBJECT | SCALAR
        directive @link(url: String, as: String, for: link__Purpose, import: [link__Import]) repeatable on SCHEMA
                
        scalar link__Import
        enum link__Purpose {
          SECURITY
          EXECUTION
        }

        type Computer
          @join__type(graph: COMPUTERS)
        {
          id: ID!
          errorField: String
          nonNullErrorField: String!
        }
        
        scalar join__FieldSet
        
        enum join__Graph {
          COMPUTERS @join__graph(name: "computers", url: "http://localhost:4001/")
        }


        type Query
          @join__type(graph: COMPUTERS)
        {
          computer(id: ID!): Computer
        }"#;

        let planner = Planner::<serde_json::Value>::new(
            schema.to_string(),
            QueryPlannerConfig {
                incremental_delivery: Some(IncrementalDeliverySupport {
                    enable_defer: Some(true),
                }),
            },
        )
        .await
        .unwrap();

        let plan_response = planner
            .plan(
                r#"query { 
                        computer(id: "Computer1") {   
                        id
                        ...ComputerErrorField @defer
                        }
                    }
                    fragment ComputerErrorField on Computer {
                        errorField
                    }"#
                .to_string(),
                None,
            )
            .await
            .unwrap()
            .data
            .unwrap();

        insta::assert_snapshot!(serde_json::to_string_pretty(&plan_response).unwrap());
    }

    #[tokio::test]
    async fn defer_query_plan() {
        let schema = r#"schema
                @core(feature: "https://specs.apollo.dev/core/v0.1")
                @core(feature: "https://specs.apollo.dev/join/v0.1")
                @core(feature: "https://specs.apollo.dev/inaccessible/v0.1")
                {
                query: Query
        }
        directive @core(feature: String!) repeatable on SCHEMA
        directive @join__field(graph: join__Graph, requires: join__FieldSet, provides: join__FieldSet) on FIELD_DEFINITION
        directive @join__type(graph: join__Graph!, key: join__FieldSet) repeatable on OBJECT | INTERFACE
        directive @join__owner(graph: join__Graph!) on OBJECT | INTERFACE
        directive @join__graph(name: String!, url: String!) on ENUM_VALUE
        directive @inaccessible on OBJECT | FIELD_DEFINITION | INTERFACE | UNION
        scalar join__FieldSet
        enum join__Graph {
            USER @join__graph(name: "user", url: "http://localhost:4001/graphql")
            ORGA @join__graph(name: "orga", url: "http://localhost:4002/graphql")
        }
        type Query {
            currentUser: User @join__field(graph: USER)
        }
        type User
        @join__owner(graph: USER)
        @join__type(graph: ORGA, key: "id")
        @join__type(graph: USER, key: "id"){
            id: ID!
            name: String
            activeOrganization: Organization
        }
        type Organization
        @join__owner(graph: ORGA)
        @join__type(graph: ORGA, key: "id")
        @join__type(graph: USER, key: "id") {
            id: ID
            creatorUser: User
            name: String
            nonNullId: ID!
            suborga: [Organization]
        }"#;

        let planner = Planner::<serde_json::Value>::new(
            schema.to_string(),
            QueryPlannerConfig {
                incremental_delivery: Some(IncrementalDeliverySupport {
                    enable_defer: Some(true),
                }),
            },
        )
        .await
        .unwrap();

        insta::assert_snapshot!(serde_json::to_string_pretty(&planner
            .plan(
                "query { currentUser { activeOrganization { id  suborga { id ...@defer { nonNullId } } } } }"
                .to_string(),
                None
            )
            .await
            .unwrap()
        .data
        .unwrap()).unwrap());
    }
}
