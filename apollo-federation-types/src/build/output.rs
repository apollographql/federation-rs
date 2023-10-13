use crate::build::BuildHint;

use serde::{Deserialize, Serialize};

/// BuildOutput contains information about the supergraph that was composed.
/// New fields added to this struct must be optional in order to maintain
/// backwards compatibility with old versions of Rover.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildOutput {
    /// Supergraph SDL can be used to start a gateway instance.
    pub supergraph_sdl: String,

    /// Hints contain information about the composition and should be displayed.
    pub hints: Vec<BuildHint>,

    /// Other untyped JSON included in the build output.
    #[serde(flatten)]
    pub other: crate::UncaughtJson,
}

impl BuildOutput {
    /// Create output containing only a supergraph schema
    pub fn new(supergraph_sdl: String) -> Self {
        Self::new_with_hints(supergraph_sdl, Vec::new())
    }

    /// Create output containing a supergraph schema and some hints
    pub fn new_with_hints(supergraph_sdl: String, hints: Vec<BuildHint>) -> Self {
        Self {
            supergraph_sdl,
            hints,
            other: crate::UncaughtJson::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    #[test]
    fn it_can_serialize_without_hints() {
        let sdl = "my-sdl".to_string();
        let expected_json = json!({"supergraphSdl": &sdl, "hints": []});
        let actual_json = serde_json::to_value(&BuildOutput::new(sdl)).unwrap();
        assert_eq!(expected_json, actual_json)
    }

    #[test]
    fn it_can_serialize_with_hints() {
        let sdl = "my-sdl".to_string();
        let hint_one = "hint-one".to_string();
        let hint_two = "hint-two".to_string();
        let code = "code".to_string();
        let code2 = "code2".to_string();
        let expected_json = json!({"supergraphSdl": &sdl, "hints": [{"message": &hint_one, "code": &code, "nodes": null}, {"message": &hint_two, "code": &code2, "nodes": null}]});
        let actual_json = serde_json::to_value(&BuildOutput::new_with_hints(
            sdl.to_string(),
            vec![
                BuildHint::new(hint_one, code, None, None),
                BuildHint::new(hint_two, code2, None, None),
            ],
        ))
        .unwrap();
        assert_eq!(expected_json, actual_json)
    }

    #[test]
    fn it_can_deserialize_without_hints() {
        let sdl = "my-sdl".to_string();
        let actual_struct =
            serde_json::from_str(&json!({"supergraphSdl": &sdl, "hints": []}).to_string()).unwrap();
        let expected_struct = BuildOutput::new(sdl);

        assert_eq!(expected_struct, actual_struct)
    }

    #[test]
    fn it_can_deserialize_with_hints() {
        let sdl = "my-sdl".to_string();
        let hint_one = "hint-one".to_string();
        let hint_two = "hint-two".to_string();
        let code = "code".to_string();
        let code2 = "code2".to_string();
        let actual_struct =
            serde_json::from_str(&json!({"supergraphSdl": &sdl, "hints": [{"message": &hint_one, "code": &code}, {"message": &hint_two, "code": &code2}]}).to_string())
                .unwrap();
        let expected_struct = BuildOutput::new_with_hints(
            sdl,
            vec![
                BuildHint::new(hint_one, code, None, None),
                BuildHint::new(hint_two, code2, None, None),
            ],
        );

        assert_eq!(expected_struct, actual_struct)
    }

    #[test]
    fn it_can_deserialize_even_with_unknown_fields() {
        let sdl = "my-sdl".to_string();
        let unexpected_key = "this-would-never-happen".to_string();
        let unexpected_value = "but-maybe-something-else-more-reasonable-would".to_string();
        let actual_struct = serde_json::from_str(
            &json!({"supergraphSdl": &sdl, "hints": [], &unexpected_key: &unexpected_value})
                .to_string(),
        )
        .unwrap();
        let mut expected_struct = BuildOutput::new(sdl);
        expected_struct
            .other
            .insert(unexpected_key, Value::String(unexpected_value));

        assert_eq!(expected_struct, actual_struct)
    }
}
