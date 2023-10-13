use crate::build::BuildErrorNode;
use serde::{Deserialize, Serialize};

/// BuildHint contains helpful information that pertains to a build
/// New fields added to this struct must be optional in order to maintain
/// backwards compatibility with old versions of Rover.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildHint {
    /// The message of the hint
    pub message: String,

    /// The code of the hint, this is an Option to maintain backwards compatibility.
    pub code: Option<String>,

    pub nodes: Option<Vec<BuildErrorNode>>,

    pub omitted_nodes_count: Option<u32>,

    /// Other untyped JSON included in the build hint.
    #[serde(flatten)]
    pub other: crate::UncaughtJson,
}

impl BuildHint {
    pub fn new(message: String, code: String, nodes: Option<Vec<BuildErrorNode>>, omitted_nodes_count: Option<u32>) -> Self {
        Self {
            message,
            code: Some(code),
            nodes,
            omitted_nodes_count,
            other: crate::UncaughtJson::new(),
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
        let code = "hintCode".to_string();
        let expected_json = json!({ "message": &msg, "code": &code, "nodes": null });
        let actual_json = serde_json::to_value(&BuildHint::new(msg, code, None, None)).unwrap();
        assert_eq!(expected_json, actual_json)
    }

    #[test]
    fn it_can_deserialize() {
        let msg = "hint".to_string();
        let code = "hintCode".to_string();
        let actual_struct = serde_json::from_str(
            &json!({ "message": &msg, "code": &code, "nodes": null, "omittedNodesCount": 12 }).to_string(),
        )
        .unwrap();
        let expected_struct = BuildHint::new(msg, code, None, Some(12));
        assert_eq!(expected_struct, actual_struct);
    }

    #[test]
    fn it_can_deserialize_even_with_unknown_fields() {
        let msg = "hint".to_string();
        let code = "hintCode".to_string();
        let unexpected_key = "this-would-never-happen".to_string();
        let unexpected_value = "but-maybe-something-else-more-reasonable-would".to_string();
        let actual_struct = serde_json::from_str(
            &json!({ "message": &msg, "code": &code, &unexpected_key: &unexpected_value })
                .to_string(),
        )
        .unwrap();
        let mut expected_struct = BuildHint::new(msg, code, None, None);
        expected_struct
            .other
            .insert(unexpected_key, Value::String(unexpected_value));
        assert_eq!(expected_struct, actual_struct);
    }
}
