/*!
# Errors raised by the `router-bridge` when trying to run `javascript`.
*/

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Serialize, Deserialize, Debug, Clone)]
/// An error which occurred within the bridge.
///
/// This does not include JS domain related errors, such as [`GraphQLError`].
pub enum Error {
    /// An uncaught error was raised when invoking a custom script.
    ///
    /// This contains the script invocation error message.
    #[error("the deno runtime raised an error: `{0}`")]
    DenoRuntime(String),
    /// An uncaught error was raised when trying to serialize a parameter before sending it to the javascript worker.
    ///
    /// This contains the serialization error message, and the payload name.
    #[error("couldn't serialize parameter `{name}`: `{message}`.")]
    ParameterSerialization {
        /// The underlying serialization error.
        message: String,
        /// The name of the parameter we tried to serialize.
        name: String,
    },

    /// An uncaught error was raised when trying to deserialize a payload.
    ///
    /// This contains the deserialization error message, and the payload.
    #[error("couldn't deserialize payload `{id}`: `{message}`.")]
    ParameterDeserialization {
        /// The underlying serialization error.
        message: String,
        /// The deno response id we tried to deserialize.
        id: String,
    },
}
