/*!
# Generate an API schema from an sdl.
*/

use crate::error::Error;
use crate::js::Js;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use thiserror::Error;

/// An error which occurred during JavaScript api schema generation.
///
/// The shape of this error is meant to mimick that of the error created within
/// JavaScript, which is a [`GraphQLError`] from the [`graphql-js`] library.
///
/// [`graphql-js']: https://npm.im/graphql
/// [`GraphQLError`]: https://github.com/graphql/graphql-js/blob/3869211/src/error/GraphQLError.js#L18-L75
#[derive(Debug, Error, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct ApiSchemaError {
    /// Whether this error occurred during validation or not.
    #[serde(default)]
    pub validation_error: bool,
    /// A human-readable description of the error that prevented api schema generation.
    pub message: Option<String>,
}

impl Display for ApiSchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message.as_deref().unwrap_or("UNKNOWN"))
    }
}

/// Options for generating the API schema.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiSchemaOptions {
    /// Whether to validate the GraphQL input string.
    pub graphql_validation: bool,
}

/// The type returned when invoking `api_schema`
pub type ApiSchemaResult = Result<String, Vec<ApiSchemaError>>;

/// The `api_schema` function receives a [`string`] representing the SDL and invokes JavaScript
/// functions to parse, convert to apiSchema and print to string.
pub fn api_schema(sdl: &str, options: ApiSchemaOptions) -> Result<ApiSchemaResult, Error> {
    Js::new("api_schema".to_string())
        .with_parameter("sdl", sdl)?
        .with_parameter("graphqlValidation", options.graphql_validation)?
        .execute::<ApiSchemaResult>("do_api_schema", include_str!("../bundled/do_api_schema.js"))
}

#[cfg(test)]
mod tests {
    use crate::api_schema::{api_schema, ApiSchemaError, ApiSchemaOptions};

    #[test]
    fn it_works() {
        let raw_sdl = include_str!("testdata/contract_schema.graphql");

        let api_schema = api_schema(
            raw_sdl,
            ApiSchemaOptions {
                graphql_validation: true,
            },
        )
        .unwrap();
        insta::assert_snapshot!(&api_schema.unwrap());
    }

    #[test]
    fn invalid_sdl() {
        let expected_error = ApiSchemaError {
            message: Some(r#"Unknown type "Query"."#.to_string()),
            validation_error: true,
        };
        let response = api_schema(
            "schema {
                query: Query
            }",
            ApiSchemaOptions {
                graphql_validation: true,
            },
        )
        .expect("an uncaught deno error occured");

        assert_eq!(response.err().unwrap(), vec![expected_error]);
    }
}
