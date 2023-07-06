use super::BuildMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// PluginResult represents the output of a plugin execution
/// New fields added to this struct must be optional in order to maintain
/// backwards compatibility with old versions of Rover
pub struct PluginResult {
    pub is_success: bool,
    pub schema: Option<String>,
    pub build_messages: Vec<BuildMessage>,

    // We need to keep the default for backwards compatibility
    #[serde(default = "bool::default")]
    is_non_build_failure: bool,

    /// Other untyped JSON included in the build output.
    #[serde(flatten)]
    other: crate::UncaughtJson,
}

impl PluginResult {
    /// Returns true if the failure is due to something other than the execution of the underlying javascript code
    /// e.g. the supergraph yaml is misconfigured or the json recieved is malformed
    pub fn is_non_build_failure(&self) -> bool {
        self.is_non_build_failure && self.schema.is_none() && !self.is_success
    }

    pub fn internal_failure_result(build_messages: Vec<BuildMessage>) -> Self {
        Self {
            schema: None,
            is_non_build_failure: true,
            is_success: false,
            build_messages,
            other: crate::UncaughtJson::new(),
        }
    }

    pub fn success_from_schema(schema: String) -> Self {
        Self {
            schema: Some(schema),
            is_non_build_failure: true,
            is_success: false,
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
        match serde_json {
            Ok(js_response) => js_response,
            Err(json_error) => {
                PluginResult::internal_failure_result(vec![BuildMessage::new_error(
                    format!(
                        "Could not parse JSON from Rust. Received error {}",
                        json_error
                    ),
                    Some("PLUGIN_EXECUTION".to_string()),
                    Some("PLUGIN_EXECUTION".to_string()),
                )])
            }
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!(self)
    }
}

#[cfg(feature = "config")]
impl From<crate::config::ConfigError> for PluginResult {
    fn from(config_error: crate::config::ConfigError) -> Self {
        PluginResult::internal_failure_result(vec![BuildMessage::new_error(
            config_error.message(),
            Some("PLUGIN_CONFIGURATION".to_string()),
            config_error.code(),
        )])
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    #[test]
    fn it_can_serialize_with_success() {
        let sdl = "my-sdl".to_string();
        let expected_json = json!({"schema": &sdl, "buildMessages": [], "isSuccess": true, "isNonBuildFailure": false});
        let actual_json = serde_json::to_value(&PluginResult {
            schema: Some(sdl),
            build_messages: vec![],
            is_success: true,
            is_non_build_failure: false,
            other: crate::UncaughtJson::new(),
        })
        .unwrap();
        assert_eq!(expected_json, actual_json)
    }

    #[test]
    fn it_can_serialize_with_failure() {
        let expected_json = json!({
        "schema": null,
        "buildMessages": [],
        "isSuccess": false,
        "isNonBuildFailure": false,
        });
        let actual_json = serde_json::to_value(&PluginResult {
            schema: None,
            build_messages: vec![],
            is_success: false,
            is_non_build_failure: false,
            other: crate::UncaughtJson::new(),
        })
        .expect("Could not serialize PluginResult");
        assert_eq!(expected_json, actual_json)
    }

    #[test]
    fn it_can_deserialize_with_success() {
        let sdl = "my-sdl".to_string();
        let actual_struct = serde_json::from_str(
            &json!({"schema": &sdl, "buildMessages": [], "isSuccess": true}).to_string(),
        )
        .unwrap();
        let expected_struct = PluginResult {
            schema: Some(sdl),
            build_messages: vec![],
            is_success: true,
            is_non_build_failure: false,
            other: crate::UncaughtJson::new(),
        };

        assert_eq!(expected_struct, actual_struct)
    }

    #[test]
    fn it_can_deserialize_with_failure() {
        let actual_struct = serde_json::from_str(
            &json!({"schema": null, "buildMessages": [], "isSuccess": false}).to_string(),
        )
        .unwrap();
        let expected_struct = PluginResult {
            schema: None,
            build_messages: vec![],
            is_success: false,
            is_non_build_failure: false,
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
            &json!({"schema": &sdl, "buildMessages": [], "isSuccess": true, &unexpected_key: &unexpected_value}).to_string(),
        )
        .unwrap();
        let mut expected_struct = PluginResult {
            schema: Some(sdl),
            build_messages: vec![],
            is_success: true,
            is_non_build_failure: false,
            other: crate::UncaughtJson::new(),
        };

        expected_struct
            .other
            .insert(unexpected_key, Value::String(unexpected_value));

        assert_eq!(expected_struct, actual_struct)
    }
}
