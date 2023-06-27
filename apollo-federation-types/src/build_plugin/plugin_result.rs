use super::BuildMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginResult {
    pub is_success: bool,
    pub schema: Option<String>,
    pub build_messages: Vec<BuildMessage>,

    /// Other untyped JSON included in the build output.
    #[serde(flatten)]
    other: crate::UncaughtJson,
}

impl PluginResult {
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
            Err(json_error) => Self {
                is_success: false,
                schema: None,
                build_messages: BuildMessage::to_build_errors(
                    vec![format!(
                        "Could not parse JSON from Rust. Received error {}",
                        json_error
                    )],
                    Some("PLUGIN_EXECUTION".to_string()),
                    Some("PLUGIN_EXECUTION".to_string()),
                ),
                other: crate::UncaughtJson::new(),
            },
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!(self)
    }
}

#[cfg(feature = "config")]
impl From<crate::config::ConfigError> for PluginResult {
    fn from(config_error: crate::config::ConfigError) -> Self {
        PluginResult {
            schema: None,
            build_messages: BuildMessage::to_build_errors(
                vec![config_error.message()],
                Some("PLUGIN_CONFIGURATION".to_string()),
                config_error.code(),
            ),
            is_success: false,
            other: crate::UncaughtJson::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    #[test]
    fn it_can_serialize_with_success() {
        let sdl = "my-sdl".to_string();
        let expected_json = json!({"schema": &sdl, "buildMessages": [], "isSuccess": true});
        let actual_json = serde_json::to_value(&PluginResult {
            schema: Some(sdl),
            build_messages: vec![],
            is_success: true,
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
        "isSuccess": false
        });
        let actual_json = serde_json::to_value(&PluginResult {
            schema: None,
            build_messages: vec![],
            is_success: false,
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
            other: crate::UncaughtJson::new(),
        };

        expected_struct
            .other
            .insert(unexpected_key, Value::String(unexpected_value));

        assert_eq!(expected_struct, actual_struct)
    }
}
