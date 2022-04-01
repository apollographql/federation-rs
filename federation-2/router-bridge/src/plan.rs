/*!
# Create a query plan
*/

use crate::error::Error;
use crate::js::Js;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use thiserror::Error;

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

/// Create the query plan by calling in to JS.
///
/// We use a generic here because the output type `QueryPlan` is part of the router.
/// Since this bridge is temporary we don't to declare the `QueryPlan` structure in this crate.
/// We will instead let the caller define what structure the plan result should be deserialized into.
pub fn plan<T: DeserializeOwned + 'static>(
    context: OperationalContext,
    options: QueryPlanOptions,
) -> Result<Result<T, BridgeErrors>, Error> {
    Js::new()
        .with_parameter("schemaString", context.schema)?
        .with_parameter("queryString", context.query)?
        .with_parameter("options", options)?
        .with_parameter("operationName", context.operation_name)?
        .execute("do_plan", include_str!("../js-dist/do_plan.js"))
        .map(|inner: Result<T, Vec<BridgeError>>| inner.map_err(|errors| BridgeErrors { errors }))
}
