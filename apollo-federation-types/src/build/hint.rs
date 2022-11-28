use serde::{Deserialize, Serialize};
use regex::Regex;
use lazy_static::lazy_static;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct BuildHintLevel {
    /// Value of the hint level. Higher values correspond to more "important" hints.
    pub value: u16,

    /// Readable name of the hint level
    pub name: String,
}

impl BuildHintLevel {
    pub fn warn() -> Self { Self { value: 60, name: String::from("WARN") } }
    pub fn info() -> Self { Self { value: 40, name: String::from("INFO") } }
    pub fn debug() -> Self { Self { value: 20, name: String::from("DEBUG") } }
}


/// BuildHint contains helpful information that pertains to a build
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct BuildHint {
    /// The message of the hint
    /// This will usually be formatted as "[<hint code>] <message details>" and the
    /// `BuildHint::extract_code_and_message` method can be used to extract the components of this
    /// message.
    pub message: String,

    /// The level of the hint
    // We should always get a level out of recent harmonizer, but older one will not have it and we
    // default to "INFO".
    #[serde(default="BuildHintLevel::info")]
    pub level: BuildHintLevel,

    /// Other untyped JSON included in the build hint.
    #[serde(flatten)]
    pub other: crate::UncaughtJson,
}

impl BuildHint {
    pub fn new(message: String, level: BuildHintLevel) -> Self {
        Self {
            message,
            level,
            other: crate::UncaughtJson::new(),
        }
    }

    pub fn debug(message: String) -> Self {
        Self::new(message, BuildHintLevel::debug())
    }

    pub fn info(message: String) -> Self {
        Self::new(message, BuildHintLevel::info())
    }

    pub fn warn(message: String) -> Self {
        Self::new(message, BuildHintLevel::warn())
    }

    /// Extracts the underlying code and "raw" message of the hint.
    pub fn extract_code_and_message(&self) -> (String, String) {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^\[(\w+)\] (.+)").unwrap();
        }
        let maybe_captures = RE.captures(&self.message);
        if let Some(captures) = maybe_captures {
            (captures.get(1).unwrap().as_str().to_string(), captures.get(2).unwrap().as_str().to_string())
        } else {
            (String::from("UNKNOWN"), self.message.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    #[test]
    fn it_can_serialize() {
        let msg = "hint".to_string();
        let expected_json = json!({"level": { "value": 40, "name": "INFO"}, "message": &msg });
        let actual_json = serde_json::to_value(&BuildHint::info(msg)).unwrap();
        assert_eq!(expected_json, actual_json)
    }

    #[test]
    fn it_can_deserialize() {
        let msg = "hint".to_string();
        let actual_struct = serde_json::from_str(&json!({"level": { "value": 20, "name": "DEBUG"},  "message": &msg }).to_string()).unwrap();
        let expected_struct = BuildHint::debug(msg);
        assert_eq!(expected_struct, actual_struct);
    }

    #[test]
    fn it_can_deserialize_without_levels() {
        let msg = "hint".to_string();
        let actual_struct = serde_json::from_str(&json!({ "message": &msg }).to_string()).unwrap();
        let expected_struct = BuildHint::info(msg);
        assert_eq!(expected_struct, actual_struct);
    }

    #[test]
    fn it_can_deserialize_even_with_unknown_fields() {
        let msg = "hint".to_string();
        let unexpected_key = "this-would-never-happen".to_string();
        let unexpected_value = "but-maybe-something-else-more-reasonable-would".to_string();
        let actual_struct = serde_json::from_str(
            &json!({ "message": &msg, &unexpected_key: &unexpected_value }).to_string(),
        )
        .unwrap();
        let mut expected_struct = BuildHint::info(msg);
        expected_struct
            .other
            .insert(unexpected_key, Value::String(unexpected_value));
        assert_eq!(expected_struct, actual_struct);
    }

    #[test]
    fn it_extracts_code_and_message() {
        let hint = BuildHint::info("[MY_CODE] Some message".to_string());
        let (actual_code, actual_message) = hint.extract_code_and_message();
        let expected_code = "MY_CODE".to_string();
        let expected_message = "Some message".to_string();
        assert_eq!(expected_code, actual_code);
        assert_eq!(expected_message, actual_message);
    }

    #[test]
    fn it_handle_extracting_code_and_message_with_unknown_code() {
        let hint = BuildHint::info("Some message without code".to_string());
        let (actual_code, actual_message) = hint.extract_code_and_message();
        let expected_code = "UNKNOWN".to_string();
        let expected_message = "Some message without code".to_string();
        assert_eq!(expected_code, actual_code);
        assert_eq!(expected_message, actual_message);
    }
}
