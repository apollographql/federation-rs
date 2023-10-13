use std::{
    error::Error,
    fmt::{self, Display},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildError {
    /// A message describing the build error.
    message: Option<String>,

    /// A code describing the build error.
    code: Option<String>,

    /// The type of build error.
    r#type: BuildErrorType,

    /// Other untyped JSON included in the build output.
    #[serde(flatten)]
    other: crate::UncaughtJson,

    nodes: Option<Vec<BuildErrorNode>>,

    omitted_nodes_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildErrorNode {
    subgraph: Option<String>,

    source: Option<String>,

    start: Option<BuildErrorNodeLocationToken>,
    end: Option<BuildErrorNodeLocationToken>,
}

impl BuildErrorNode {
    pub fn get_subgraph(&self) -> Option<String> {
        self.subgraph.clone()
    }

    pub fn get_source(&self) -> Option<String> {
        self.source.clone()
    }

    pub fn get_start(&self) -> Option<BuildErrorNodeLocationToken> {
        self.start.clone()
    }

    pub fn get_end(&self) -> Option<BuildErrorNodeLocationToken> {
        self.end.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildErrorNodeLocation {
    subgraph: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildErrorNodeLocationToken {
    start: Option<u32>,
    end: Option<u32>,
    column: Option<u32>,
    line: Option<u32>,
}

impl BuildErrorNodeLocationToken {
    pub fn get_start(&self) -> Option<u32> {
        self.start
    }

    pub fn get_end(&self) -> Option<u32> {
        self.end
    }

    pub fn get_column(&self) -> Option<u32> {
        self.column
    }

    pub fn get_line(&self) -> Option<u32> {
        self.line
    }
}

impl BuildError {
    pub fn composition_error(
        code: Option<String>,
        message: Option<String>,
        nodes: Option<Vec<BuildErrorNode>>,
        omitted_nodes_count: Option<u32>,
    ) -> BuildError {
        BuildError::new(code, message, BuildErrorType::Composition, nodes, omitted_nodes_count)
    }

    pub fn config_error(code: Option<String>, message: Option<String>) -> BuildError {
        BuildError::new(code, message, BuildErrorType::Config, None, None)
    }

    fn new(
        code: Option<String>,
        message: Option<String>,
        r#type: BuildErrorType,
        nodes: Option<Vec<BuildErrorNode>>,
        omitted_nodes_count: Option<u32>,
    ) -> BuildError {
        let real_message = if code.is_none() && message.is_none() {
            Some("An unknown error occurred during the build.".to_string())
        } else {
            message
        };
        BuildError {
            code,
            message: real_message,
            r#type,
            other: crate::UncaughtJson::new(),
            nodes,
            omitted_nodes_count,
        }
    }

    pub fn get_message(&self) -> Option<String> {
        self.message.clone()
    }

    pub fn get_code(&self) -> Option<String> {
        self.code.clone()
    }

    pub fn get_type(&self) -> BuildErrorType {
        self.r#type.clone()
    }

    pub fn get_nodes(&self) -> Option<Vec<BuildErrorNode>> {
        self.nodes.clone()
    }

    pub fn get_omitted_nodes_count(&self) -> Option<u32> { self.omitted_nodes_count.clone() }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum BuildErrorType {
    Composition,
    Config,
}

impl Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.code.as_ref().map_or("UNKNOWN", String::as_str)
        )?;
        if let Some(message) = &self.message {
            write!(f, ": {message}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize, Default, Clone, PartialEq, Eq)]
pub struct BuildErrors {
    build_errors: Vec<BuildError>,

    #[serde(skip)]
    pub is_config: bool,
}

impl BuildErrors {
    pub fn new() -> Self {
        BuildErrors {
            build_errors: Vec::new(),
            is_config: false,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &BuildError> {
        self.build_errors.iter()
    }

    pub fn len(&self) -> usize {
        self.build_errors.len()
    }

    pub fn length_string(&self) -> String {
        let num_failures = self.build_errors.len();
        if num_failures == 0 {
            unreachable!("No build errors were encountered while composing the supergraph.");
        }

        match num_failures {
            1 => "1 build error".to_string(),
            _ => format!("{num_failures} build errors"),
        }
    }

    pub fn push(&mut self, error: BuildError) {
        if matches!(error.r#type, BuildErrorType::Config) {
            self.is_config = true;
        }
        self.build_errors.push(error);
    }

    pub fn is_empty(&self) -> bool {
        self.build_errors.is_empty()
    }
}

impl Display for BuildErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let num_failures = self.build_errors.len();
        if num_failures == 0
            || (num_failures == 1
                && self.build_errors[0].code.is_none()
                && self.build_errors[0].message.is_none())
        {
            writeln!(f, "Something went wrong! No build errors were recorded, but we also were unable to build a valid supergraph.")?;
        } else {
            for build_error in &self.build_errors {
                writeln!(f, "{build_error}")?;
            }
        }
        Ok(())
    }
}

#[cfg(feature = "config")]
impl From<crate::config::ConfigError> for BuildErrors {
    fn from(config_error: crate::config::ConfigError) -> Self {
        BuildErrors {
            build_errors: vec![BuildError::config_error(
                config_error.code(),
                Some(config_error.message()),
            )],
            is_config: true,
        }
    }
}

impl From<Vec<BuildError>> for BuildErrors {
    fn from(build_errors: Vec<BuildError>) -> Self {
        let is_config = build_errors
            .iter()
            .any(|e| matches!(e.r#type, BuildErrorType::Config));
        BuildErrors {
            build_errors,
            is_config,
        }
    }
}

impl FromIterator<BuildError> for BuildErrors {
    fn from_iter<I: IntoIterator<Item = BuildError>>(iter: I) -> Self {
        let mut c = BuildErrors::new();

        for i in iter {
            c.push(i);
        }

        c
    }
}

impl Error for BuildError {}
impl Error for BuildErrors {}

#[cfg(test)]
mod tests {
    use super::{BuildError, BuildErrors};

    use serde_json::{json, Value};
    use crate::build::{BuildErrorNode, BuildHint};

    #[test]
    fn it_supports_iter() {
        let build_errors: BuildErrors = vec![
            BuildError::composition_error(None, Some("wow".to_string()), None, None),
            BuildError::composition_error(Some("BOO".to_string()), Some("boo".to_string()), None, None),
        ]
        .into();

        let messages: Vec<String> = build_errors
            .iter()
            .map(|e| e.get_message().unwrap())
            .collect();

        assert_eq!(messages, vec!["wow", "boo"]);
    }

    #[test]
    fn it_can_serialize_empty_errors() {
        let build_errors = BuildErrors::new();
        assert_eq!(
            serde_json::to_string(&build_errors).expect("Could not serialize build errors"),
            json!({"build_errors": []}).to_string()
        );
    }

    #[test]
    fn it_can_serialize_some_build_errors() {
        let error_node = BuildErrorNode { subgraph: Some("foo".to_string()), source: None, start: None, end: None };

        let build_errors: BuildErrors = vec![
            BuildError::composition_error(None, Some("wow".to_string()), Some(vec![error_node.clone()]), Some(1)),
            BuildError::composition_error(Some("BOO".to_string()), Some("boo".to_string()), Some(vec![error_node.clone()]), Some(2)),
        ]
        .into();

        let actual_value: Value = serde_json::from_str(
            &serde_json::to_string(&build_errors)
                .expect("Could not convert build errors to string"),
        )
        .expect("Could not convert build error string to serde_json::Value");

        let expected_value = json!({
            "build_errors": [
              {
                "message": "wow",
                "code": null,
                "type": "composition",
                "nodes": [
                  {
                      "subgraph": "foo",
                      "source": null,
                      "start": null,
                      "end": null
                  }
                ],
                "omitted_nodes_count": 1
              },
              {
                "message": "boo",
                "code": "BOO",
                "type": "composition",
                "nodes": [
                  {
                      "subgraph": "foo",
                      "source": null,
                      "start": null,
                      "end": null
                  }
                ],
                "omitted_nodes_count": 2
              }
            ]
        });
        assert_eq!(actual_value, expected_value);
    }

    #[test]
    fn it_can_deserialize() {
        let msg = "wow".to_string();
        let code = "boo".to_string();
        let actual_struct = serde_json::from_str(
            &json!({ "message": &msg, "code": &code, "type": "composition", "nodes": null, "omittedNodesCount": 12 }).to_string(),
        ).unwrap();
        let expected_struct = BuildError::composition_error(Some(code.clone()), Some(msg.clone()), None, Some(12));
        assert_eq!(expected_struct, actual_struct);
    }
}
