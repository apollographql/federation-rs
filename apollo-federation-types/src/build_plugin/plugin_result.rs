use super::BuildMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
// This represents the reason for a build failrue
pub enum PluginFailureReason {
    /// If the plugin failed because user inputs can't be built
    Build,
    /// If the configuration sent to the plugin is invalid
    Config,
    /// If the plugin failed for some internal reason
    InternalFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// PluginResult represents the output of a plugin execution
/// New fields added to this struct must be optional in order to maintain
/// backwards compatibility with old versions of Rover
pub struct PluginResult {
    pub result: Result<String, PluginFailureReason>,
    pub build_messages: Vec<BuildMessage>,

    /// Other untyped JSON included in the build output.
    #[serde(flatten)]
    other: crate::UncaughtJson,
}

impl PluginResult {
    pub fn new(
        result: Result<String, PluginFailureReason>,
        build_messages: Vec<BuildMessage>,
    ) -> Self {
        Self {
            result,
            build_messages,
            other: crate::UncaughtJson::new(),
        }
    }

    pub fn new_failure(
        build_messages: Vec<BuildMessage>,
        execution_failure: PluginFailureReason,
    ) -> Self {
        Self {
            result: Err(execution_failure),
            build_messages,
            other: crate::UncaughtJson::new(),
        }
    }

    pub fn success_from_schema(schema: String) -> Self {
        Self {
            result: Ok(schema),
            build_messages: vec![],
            other: crate::UncaughtJson::new(),
        }
    }

    /**
    We may succed in Rust's perspective, but inside the JSON message may be isSuccess: false
    and buildMessages from composition telling us what went wrong.

    If there are, promote those to more semantic places in the output object.
    If there are not, cooool, pass the data along.
    */
    pub fn from_plugin_result(result_json: &str) -> Self {
        let serde_json: Result<PluginResult, serde_json::Error> = serde_json::from_str(result_json);
        serde_json.unwrap_or_else(|json_error| PluginResult::new_failure(
            vec![BuildMessage::new_error(
                format!(
                    "Could not parse JSON from Rust. Received error {json_error}"
                ),
                Some("PLUGIN_EXECUTION".to_string()),
                Some("PLUGIN_EXECUTION".to_string()),
            )],
            PluginFailureReason::InternalFailure,
        ))
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!(self)
    }
}

#[cfg(feature = "config")]
impl From<crate::config::ConfigError> for PluginResult {
    fn from(config_error: crate::config::ConfigError) -> Self {
        PluginResult::new_failure(
            vec![BuildMessage::new_error(
                config_error.message(),
                Some("PLUGIN_CONFIGURATION".to_string()),
                config_error.code(),
            )],
            PluginFailureReason::Config,
        )
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    #[test]
    fn it_can_serialize_with_success() {
        let sdl = "my-sdl".to_string();
        let expected_json = json!({"result":{ "Ok": &sdl}, "buildMessages": []});
        let actual_json = serde_json::to_value(PluginResult {
            result: Ok(sdl),
            build_messages: vec![],
            other: crate::UncaughtJson::new(),
        })
        .unwrap();
        assert_eq!(expected_json, actual_json)
    }

    #[test]
    fn it_can_serialize_with_failure() {
        let expected_json = json!({
        "result": {"Err": "build"},
        "buildMessages": [],
        });
        let actual_json = serde_json::to_value(PluginResult {
            result: Err(PluginFailureReason::Build),
            build_messages: vec![],
            other: crate::UncaughtJson::new(),
        })
        .expect("Could not serialize PluginResult");
        assert_eq!(expected_json, actual_json)
    }

    #[test]
    fn it_can_deserialize_with_success() {
        let sdl = "my-sdl".to_string();
        let actual_struct =
            serde_json::from_str(&json!({"result": {"Ok": &sdl}, "buildMessages": []}).to_string())
                .unwrap();
        let expected_struct = PluginResult {
            result: Ok(sdl),
            build_messages: vec![],
            other: crate::UncaughtJson::new(),
        };

        assert_eq!(expected_struct, actual_struct)
    }

    #[test]
    fn it_can_deserialize_with_failure() {
        let actual_struct = serde_json::from_str(
            &json!({"result": {"Err": "build"}, "buildMessages": []}).to_string(),
        )
        .unwrap();
        let expected_struct = PluginResult {
            result: Err(PluginFailureReason::Build),
            build_messages: vec![],
            other: crate::UncaughtJson::new(),
        };

        assert_eq!(expected_struct, actual_struct)
    }

    #[test]
    fn it_can_deserialize_even_with_unknown_fields() {
        let sdl = "my-sdl".to_string();
        let unexpected_key = "this-would-never-happen".to_string();
        let unexpected_value = "but-maybe-something-else-more-reasonable-would".to_string();
        let actual_struct = serde_json::from_str(
            &json!({"result": {"Ok": &sdl}, "buildMessages": [], &unexpected_key: &unexpected_value}).to_string(),
        )
        .unwrap();
        let mut expected_struct = PluginResult {
            result: Ok(sdl),
            build_messages: vec![],
            other: crate::UncaughtJson::new(),
        };

        expected_struct
            .other
            .insert(unexpected_key, Value::String(unexpected_value));

        assert_eq!(expected_struct, actual_struct)
    }
}
