/*!
 * Instantiate a QueryPlanner from a schema, and perform query planning
*/

use crate::worker::JsWorker;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use thiserror::Error;

// ------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// Options for the query plan
pub struct QueryPlanOptions {
    /// Use auto fragmentation
    pub auto_fragmentization: bool,
}

/// Default options for query planning
impl QueryPlanOptions {
    /// Default query plan options
    pub fn default() -> QueryPlanOptions {
        QueryPlanOptions {
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

#[derive(Debug, Error, Serialize, Deserialize, PartialEq)]
/// Container for planning errors
pub struct BridgeErrors {
    /// The contained errors
    pub errors: Vec<BridgeError>,
}

impl Display for BridgeErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Planning errors: {}",
            self.errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        ))
    }
}

/// An error which occurred during JavaScript planning.
///
/// The shape of this error is meant to mimic that of the error created within
/// JavaScript, which is a [`GraphQLError`] from the [`graphql-js`] library.
///
/// [`graphql-js`]: https://npm.im/graphql
/// [`GraphQLError`]: https://github.com/graphql/graphql-js/blob/3869211/src/error/GraphQLError.js#L18-L75
#[derive(Debug, Error, Serialize, Deserialize, PartialEq)]
pub struct BridgeError {
    /// A human-readable description of the error that prevented planning.
    pub message: Option<String>,
    /// [`BridgeErrorExtensions`]
    #[serde(deserialize_with = "none_only_if_value_is_null_or_empty_object")]
    pub extensions: Option<BridgeErrorExtensions>,
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
                "invalid neither null nor empty object: found {:?}",
                obj,
            )))
        }
        Err(e) => Err(e),
    }
}

impl Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(msg) = &self.message {
            f.write_fmt(format_args!("{code}: {msg}", code = self.code(), msg = msg))
        } else {
            f.write_str(self.code())
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
/// Error codes
pub struct BridgeErrorExtensions {
    /// The error code
    pub code: String,
}

/// An error that was received during planning within JavaScript.
impl BridgeError {
    /// Retrieve the error code from an error received during planning.
    pub fn code(&self) -> &str {
        match self.extensions {
            Some(ref ext) => &*ext.code,
            None => "UNKNOWN",
        }
    }
}

// ------------------------------------

#[derive(Deserialize, Debug)]
/// The result of a router bridge invocation
pub struct BridgeResult<T> {
    /// The data if the query was successfully run
    pub data: Option<T>,
    /// The errors if the query failed
    pub errors: Option<Vec<BridgeError>>,
}

impl<T> BridgeResult<T>
where
    T: DeserializeOwned + Send + Debug + 'static,
{
    /// Turn a BridgeResult into an actual Result
    pub fn into_result(self) -> Result<T, Vec<BridgeError>> {
        if let Some(data) = self.data {
            Ok(data)
        } else {
            Err(self.errors.unwrap_or_else(|| {
                vec![BridgeError {
                    message: Some("an unknown error occured".to_string()),
                    extensions: None,
                }]
            }))
        }
    }
}

/// A Deno worker backed query Planner.

pub struct Planner<T>
where
    T: DeserializeOwned + Send + Debug + 'static,
{
    worker: Arc<JsWorker<PlanCmd, BridgeResult<T>>>,
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
    pub async fn new(schema: String) -> Result<Self, Vec<BridgeError>> {
        let worker =
            JsWorker::<PlanCmd, BridgeResult<T>>::new(include_str!("../js-dist/plan_worker.js"));
        let worker_is_set_up = worker
            .request(PlanCmd::UpdateSchema { schema })
            .await
            .map_err(|e| {
                vec![BridgeError {
                    message: Some(e.to_string()),
                    extensions: None,
                }]
            });

        // Both cases below the mean schema update failed.
        // We need to pay attention here.
        // returning early will drop the worker, which will join the jsruntime thread.
        // however the event loop will run for ever. We need to let the worker know it needs to exit,
        // before we drop the worker
        match worker_is_set_up {
            Err(setup_error) => {
                let _ = worker.request(PlanCmd::Exit).await;
                return Err(setup_error);
            }
            Ok(setup) => {
                if let Some(errors) = setup.errors {
                    let _ = worker.request(PlanCmd::Exit).await;
                    return Err(errors);
                }
            }
        }

        let worker = Arc::new(worker);

        Ok(Self { worker })
    }

    /// Plan a query against an instantiated query planner
    pub async fn plan(
        &self,
        query: String,
        operation_name: Option<String>,
    ) -> Result<BridgeResult<T>, crate::error::Error> {
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

        let data = planner
            .plan(QUERY.to_string(), None)
            .await
            .unwrap()
            .data
            .unwrap();

        insta::assert_snapshot!(serde_json::to_string_pretty(&data).unwrap());
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
        let errors= vec![BridgeError {
                message: Some("Cannot spread fragment \"thatUserFragment1\" within itself via \"thatUserFragment2\".".to_string()),
                extensions: None,
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
        let errors = vec![BridgeError {
            message: Some(
                "Field \"id\" must not have a selection since type \"ID!\" has no subfields."
                    .to_string(),
            ),
            extensions: None,
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
        let errors = vec![BridgeError {
            message: Some("Fragment \"UnusedTestFragment\" is never used.".to_string()),
            extensions: None,
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
        let errors = vec![BridgeError {
            message: Some(
                "Must provide operation name if query contains multiple operations.".to_string(),
            ),
            extensions: None,
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
    async fn invalid_deserialization_doesnt_panic() {
        let planner = Planner::<serde_json::Number>::new(SCHEMA.to_string()).await;

        dbg!(&planner);
        assert!(planner.is_err());
    }

    #[tokio::test]
    async fn invalid_schema_is_caught() {
        let expected_errors = vec![BridgeError {
            message: Some("Syntax Error: Unexpected Name \"Garbage\".".to_string()),
            extensions: None,
        }];

        let actual_error = Planner::<serde_json::Value>::new("Garbage".to_string())
            .await
            .unwrap_err();

        assert_eq!(expected_errors, actual_error);
    }

    #[tokio::test]
    async fn syntactically_incorrect_query_is_caught() {
        let errors = vec![BridgeError {
            message: Some("Syntax Error: Unexpected Name \"Garbage\".".to_string()),
            extensions: None,
        }];

        assert_errors(errors, "Garbage".to_string(), None).await;
    }

    #[tokio::test]
    async fn query_missing_subfields() {
        let expected_error_message = r#"Field "reviews" of type "[Review]" must have a selection of subfields. Did you mean "reviews { ... }"?"#;

        let errors = vec![BridgeError {
            message: Some(expected_error_message.to_string()),
            extensions: None,
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
        let errors = vec![BridgeError {
            message: Some(expected_error_message.to_string()),
            extensions: None,
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
        expected_errors: Vec<BridgeError>,
        query: String,
        operation_name: Option<String>,
    ) {
        let planner = Planner::<serde_json::Value>::new(SCHEMA.to_string())
            .await
            .unwrap();

        let actual = planner.plan(query, operation_name).await.unwrap();

        assert_eq!(expected_errors, actual.errors.unwrap());
    }
}

#[cfg(test)]
mod planning_error {
    use crate::planner::{BridgeError, BridgeErrorExtensions};

    #[test]
    #[should_panic(
        expected = "Result::unwrap()` on an `Err` value: Error(\"missing field `extensions`\", line: 1, column: 2)"
    )]
    fn deserialize_empty_planning_error() {
        let raw = "{}";
        serde_json::from_str::<BridgeError>(raw).unwrap();
    }

    #[test]
    #[should_panic(
        expected = "Result::unwrap()` on an `Err` value: Error(\"missing field `extensions`\", line: 1, column: 44)"
    )]
    fn deserialize_planning_error_missing_extension() {
        let raw = r#"{ "message": "something terrible happened" }"#;
        serde_json::from_str::<BridgeError>(raw).unwrap();
    }

    #[test]
    fn deserialize_planning_error_with_extension() {
        let raw = r#"{
            "message": "something terrible happened",
            "extensions": {
                "code": "E_TEST_CASE"
            }
        }"#;

        let expected = BridgeError {
            message: Some("something terrible happened".to_string()),
            extensions: Some(BridgeErrorExtensions {
                code: "E_TEST_CASE".to_string(),
            }),
        };

        assert_eq!(expected, serde_json::from_str(raw).unwrap());
    }

    #[test]
    fn deserialize_planning_error_with_empty_object_extension() {
        let raw = r#"{
            "extensions": {}
        }"#;
        let expected = BridgeError {
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
        let expected = BridgeError {
            message: None,
            extensions: None,
        };

        assert_eq!(expected, serde_json::from_str(raw).unwrap());
    }
}
