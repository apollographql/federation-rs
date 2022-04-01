/*!
 * Instantiate a QueryPlanner from a schema, and perform query planning
*/

use crate::worker::JsWorker;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::sync::Arc;

// ------------------------------------

#[derive(Deserialize, Debug)]
/// The result of a router bridge invocation
pub struct BridgeResult<T, E> {
    /// The data if the query was successfully run
    pub data: Option<T>,
    /// The errors if the query failed
    pub errors: Option<E>,
}

/// A Deno worker backed query Planner.

pub struct Planner<T, E>
where
    T: DeserializeOwned + Send + Debug + 'static,
    E: DeserializeOwned + Send + Debug + 'static,
{
    worker: Arc<JsWorker<PlanCmd, BridgeResult<T, E>>>,
}

impl<T, E> Debug for Planner<T, E>
where
    T: DeserializeOwned + Send + Debug + 'static,
    E: DeserializeOwned + Send + Debug + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Planner").finish()
    }
}

impl<T, E> Planner<T, E>
where
    T: DeserializeOwned + Send + Debug + 'static,
    E: DeserializeOwned + Send + Debug + 'static,
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
    ) -> Result<BridgeResult<T, E>, crate::error::Error> {
        self.worker
            .request(PlanCmd::Plan {
                query,
                operation_name,
            })
            .await
    }
}

impl<T, E> Drop for Planner<T, E>
where
    T: DeserializeOwned + Send + Debug + 'static,
    E: DeserializeOwned + Send + Debug + 'static,
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
    use crate::plan::PlanningError;

    use super::*;

    const QUERY: &str = include_str!("testdata/query.graphql");
    const SCHEMA: &str = include_str!("testdata/schema.graphql");

    #[tokio::test]
    async fn it_works() {
        let planner = Planner::<serde_json::Value, serde_json::Value>::new(SCHEMA.to_string())
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
        let errors= vec![PlanningError {
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
        let errors = vec![PlanningError {
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
        let errors = vec![PlanningError {
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
        let errors = vec![PlanningError {
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
        let planner = Planner::<serde_json::Number, serde_json::Number>::new(SCHEMA.to_string())
            .await
            .unwrap();

        let _ = planner.plan(QUERY.to_string(), None).await.unwrap();

        // TODO: maybe make it an actual error?
        // dbg!(&actual);
        // assert!(actual.errors.is_some());
    }

    #[tokio::test]
    async fn invalid_schema_is_caught() {
        let errors = vec![PlanningError {
            message: Some("Syntax Error: Unexpected Name \"Garbage\".".to_string()),
            extensions: None,
        }];

        let planner = Planner::<serde_json::Value, Vec<PlanningError>>::new("Garbage".to_string())
            .await
            .unwrap();

        let actual = planner.plan(QUERY.to_string(), None).await.unwrap();

        assert_eq!(errors, actual.errors.unwrap());
    }

    // #[test]
    // fn syntactically_incorrect_query_is_caught() {
    //     let result = Err::<serde_json::Value, _>(PlanningErrors {
    //         errors: vec![PlanningError {
    //             message: Some("Syntax Error: Unexpected Name \"Garbage\".".to_string()),
    //             extensions: None,
    //         }],
    //     });
    //     assert_eq!(
    //         result,
    //         plan(
    //             OperationalContext {
    //                 schema: SCHEMA.to_string(),
    //                 query: "Garbage".to_string(),
    //                 operation_name: "".to_string(),
    //             },
    //             QueryPlanOptions::default(),
    //         )
    //         .unwrap()
    //     );
    // }

    // #[test]
    // fn query_missing_subfields() {
    //     let expected_error_message = r#"Field "reviews" of type "[Review]" must have a selection of subfields. Did you mean "reviews { ... }"?"#;

    //     let result = Err::<serde_json::Value, _>(PlanningErrors {
    //         errors: vec![PlanningError {
    //             message: Some(expected_error_message.to_string()),
    //             extensions: None,
    //         }],
    //     });
    //     // This query contains reviews, which requires subfields
    //     let query_missing_subfields = "query ExampleQuery { me { id reviews } }".to_string();
    //     assert_eq!(
    //         result,
    //         plan(
    //             OperationalContext {
    //                 schema: SCHEMA.to_string(),
    //                 query: query_missing_subfields,
    //                 operation_name: "".to_string(),
    //             },
    //             QueryPlanOptions::default(),
    //         )
    //         .unwrap()
    //     );
    // }

    // #[test]
    // fn query_field_that_doesnt_exist() {
    //     let expected_error_message = r#"Cannot query field "thisDoesntExist" on type "Query"."#;
    //     let result = Err::<serde_json::Value, _>(PlanningErrors {
    //         errors: vec![PlanningError {
    //             message: Some(expected_error_message.to_string()),
    //             extensions: None,
    //         }],
    //     });
    //     // This query contains reviews, which requires subfields
    //     let query_missing_subfields = "query ExampleQuery { thisDoesntExist }".to_string();
    //     assert_eq!(
    //         result,
    //         plan(
    //             OperationalContext {
    //                 schema: SCHEMA.to_string(),
    //                 query: query_missing_subfields,
    //                 operation_name: "".to_string(),
    //             },
    //             QueryPlanOptions::default(),
    //         )
    //         .unwrap()
    //     );
    // }

    async fn assert_errors<E>(expected_errors: E, query: String, operation_name: Option<String>)
    where
        E: Debug + DeserializeOwned + PartialEq + Send + 'static,
    {
        let planner = Planner::<serde_json::Value, E>::new(SCHEMA.to_string())
            .await
            .unwrap();

        let actual = planner.plan(query, operation_name).await.unwrap();

        assert_eq!(expected_errors, actual.errors.unwrap());
    }
}

// #[cfg(test)]
// mod planning_error {
//     use super::{PlanningError, PlanningErrorExtensions};

//     #[test]
//     #[should_panic(
//         expected = "Result::unwrap()` on an `Err` value: Error(\"missing field `extensions`\", line: 1, column: 2)"
//     )]
//     fn deserialize_empty_planning_error() {
//         let raw = "{}";
//         serde_json::from_str::<PlanningError>(raw).unwrap();
//     }

//     #[test]
//     #[should_panic(
//         expected = "Result::unwrap()` on an `Err` value: Error(\"missing field `extensions`\", line: 1, column: 44)"
//     )]
//     fn deserialize_planning_error_missing_extension() {
//         let raw = r#"{ "message": "something terrible happened" }"#;
//         serde_json::from_str::<PlanningError>(raw).unwrap();
//     }

//     #[test]
//     fn deserialize_planning_error_with_extension() {
//         let raw = r#"{
//             "message": "something terrible happened",
//             "extensions": {
//                 "code": "E_TEST_CASE"
//             }
//         }"#;

//         let expected = PlanningError {
//             message: Some("something terrible happened".to_string()),
//             extensions: Some(PlanningErrorExtensions {
//                 code: "E_TEST_CASE".to_string(),
//             }),
//         };

//         assert_eq!(expected, serde_json::from_str(raw).unwrap());
//     }

//     #[test]
//     fn deserialize_planning_error_with_empty_object_extension() {
//         let raw = r#"{
//             "extensions": {}
//         }"#;
//         let expected = PlanningError {
//             message: None,
//             extensions: None,
//         };

//         assert_eq!(expected, serde_json::from_str(raw).unwrap());
//     }

//     #[test]
//     fn deserialize_planning_error_with_null_extension() {
//         let raw = r#"{
//             "extensions": null
//         }"#;
//         let expected = PlanningError {
//             message: None,
//             extensions: None,
//         };

//         assert_eq!(expected, serde_json::from_str(raw).unwrap());
//     }
// }
