/*!
# Run introspection against a GraphQL schema and obtain the result
*/

use crate::error::Error;
use crate::js::Js;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use thiserror::Error;

/// An error which occurred during JavaScript validation.
///
/// The shape of this error is meant to mimick that of the error created within
/// JavaScript, which is a [`GraphQLError`] from the [`graphql-js`] library.
///
/// [`graphql-js']: https://npm.im/graphql
/// [`GraphQLError`]: https://github.com/graphql/graphql-js/blob/3869211/src/error/GraphQLError.js#L18-L75
#[derive(Debug, Error, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct ValidationError {
    /// A human-readable description of the error that prevented introspection.
    pub message: Option<String>,
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message.as_deref().unwrap_or("UNKNOWN"))
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct ValidationResponse {
    /// The introspection response if batch_introspect succeeded
    #[serde(default)]
    data: Option<serde_json::Value>,
    /// The errors raised on this specific query if any
    #[serde(default)]
    errors: Option<Vec<ValidationError>>,
}

pub fn validate(schema: &str, query: &str) -> Result<serde_json::Value, Error> {
    Js::new("validate".to_string())
        .with_parameter("schema", schema)?
        .with_parameter("query", query)?
        .execute::<serde_json::Value>("validate", include_str!("../bundled/do_validate.js"))
}

#[cfg(test)]
mod tests {
    use crate::validate::validate;

    #[test]
    fn it_works() {
        let schema = r#"
        type Query {
          hello: String
        }
        "#;

        let query = r#"
        {
          me
        }
        "#;

        let validated = validate(schema, query).unwrap();
        dbg!("{}", &validated);
        // assert_eq!(validatedwrap().len(), 1);
        panic!("no")
        // insta::assert_snapshot!(serde_json::to_string(&validated).unwrap());
    }
}
