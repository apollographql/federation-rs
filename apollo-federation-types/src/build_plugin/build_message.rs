use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum BuildMessageLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// BuildLocation represents the location of a build message in the GraphQLDoucment
/// New fields added to this struct must be optional in order to maintain
/// backwards compatibility with old versions of Rover
pub struct BuildMessageLocation {
    pub subgraph: Option<String>,

    pub source: Option<String>,

    pub start: Option<BuildMessagePoint>,
    pub end: Option<BuildMessagePoint>,

    #[serde(flatten)]
    pub other: crate::UncaughtJson,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildMessagePoint {
    pub start: Option<usize>,
    pub end: Option<usize>,
    pub column: Option<usize>,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// BuildMessages contains the log output of a build
/// New fields added to this struct must be optional in order to maintain
/// backwards compatibility
pub struct BuildMessage {
    pub level: BuildMessageLevel,
    pub message: String,
    pub step: Option<String>,
    pub code: Option<String>,
    pub locations: Vec<BuildMessageLocation>,
    pub schema_coordinate: Option<String>,

    #[serde(flatten)]
    pub other: crate::UncaughtJson,
}

impl BuildMessage {
    pub fn new_error(error_message: String, step: Option<String>, code: Option<String>) -> Self {
        BuildMessage {
            level: BuildMessageLevel::Error,
            message: error_message,
            step,
            code,
            locations: vec![],
            schema_coordinate: None,
            other: crate::UncaughtJson::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::build_plugin::{
        build_message::BuildMessageLocation, BuildMessage, BuildMessageLevel,
    };

    #[test]
    fn it_can_serialize_build_message() {
        let build_message: BuildMessage = BuildMessage {
            level: BuildMessageLevel::Debug,
            message: "wow".to_string(),
            step: None,
            code: None,
            locations: vec![],
            schema_coordinate: None,
            other: crate::UncaughtJson::new(),
        };

        let actual_value: Value = serde_json::from_str(
            &serde_json::to_string(&build_message)
                .expect("Could not convert build errors to string"),
        )
        .expect("Could not convert build error string to serde_json::Value");

        let expected_value = json!({
            "level": "DEBUG",
            "message": "wow",
            "step": null,
            "code": null,
            "locations": [],
            "schemaCoordinate": null,
        });
        assert_eq!(actual_value, expected_value);
    }

    #[test]
    fn it_can_deserialize_even_with_unknown_fields() {
        let unexpected_key = "this-would-never-happen".to_string();
        let unexpected_value = "but-maybe-something-else-more-reasonable-would".to_string();
        let actual_struct = serde_json::from_str(
            &json!({
                "level": "DEBUG",
                "message": "wow",
                "step": null,
                "code": null,
                "locations": [{&unexpected_key: &unexpected_value}],
                "schemaCoordinate": null,
                &unexpected_key: &unexpected_value,
            })
            .to_string(),
        )
        .unwrap();

        let mut expected_struct: BuildMessage = BuildMessage {
            level: BuildMessageLevel::Debug,
            message: "wow".to_string(),
            step: None,
            code: None,
            locations: vec![BuildMessageLocation {
                subgraph: None,
                source: None,
                start: None,
                end: None,
                other: crate::UncaughtJson::new(),
            }],
            schema_coordinate: None,
            other: crate::UncaughtJson::new(),
        };

        expected_struct.locations[0].other.insert(
            unexpected_key.clone(),
            Value::String(unexpected_value.clone()),
        );

        expected_struct
            .other
            .insert(unexpected_key, Value::String(unexpected_value));

        assert_eq!(expected_struct, actual_struct)
    }
}
