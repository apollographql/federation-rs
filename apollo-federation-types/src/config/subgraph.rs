use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

/// Config for a single [subgraph](https://www.apollographql.com/docs/federation/subgraphs/)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgraphConfig {
    /// The routing URL for the subgraph.
    /// This will appear in supergraph SDL and
    /// instructs the graph router to send all requests
    /// for this subgraph to this URL.
    pub routing_url: Option<String>,

    /// The location of the subgraph's SDL
    pub schema: SchemaSource,
}

impl SubgraphConfig {
    /// Returns SDL from the configuration file if it exists.
    /// Returns None if the configuration does not include raw SDL.
    pub fn get_sdl(&self) -> Option<String> {
        if let SchemaSource::Sdl { sdl } = &self.schema {
            Some(sdl.to_owned())
        } else {
            None
        }
    }
}

/// Options for getting SDL:
/// the graph registry, a file, or an introspection URL.
///
/// NOTE: Introspection strips all comments and directives
/// from the SDL.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
// this is untagged, meaning its fields will be flattened into the parent
// struct when de/serialized. There is no top level `schema_source`
// in the configuration.
#[serde(untagged)]
pub enum SchemaSource {
    File {
        file: Utf8PathBuf,
    },
    SubgraphIntrospection {
        subgraph_url: Url,
        introspection_headers: Option<HashMap<String, String>>,
    },
    Subgraph {
        graphref: String,
        subgraph: String,
    },
    Sdl {
        sdl: String,
    },
}

#[cfg(test)]
mod test_schema_source {
    use crate::config::SchemaSource;
    use serde_yaml::from_str;

    #[test]
    fn test_file() {
        let yaml = "file: some/path.thing";
        let source: SchemaSource = from_str(yaml).unwrap();
        let expected = SchemaSource::File {
            file: "some/path.thing".into(),
        };
        assert_eq!(source, expected);
    }

    #[test]
    fn test_subgraph_introspection_no_headers() {
        let yaml = "subgraph_url: https://example.com/graphql";
        let source: SchemaSource = from_str(yaml).unwrap();
        let expected = SchemaSource::SubgraphIntrospection {
            subgraph_url: "https://example.com/graphql".parse().unwrap(),
            introspection_headers: None,
        };
        assert_eq!(source, expected);
    }

    #[test]
    fn test_subgraph_introspection_with_headers() {
        let yaml = r#"
subgraph_url: https://example.com/graphql
introspection_headers:
    Router-Authorization: ${env.HELLO_TESTS}
    "#;
        let source: SchemaSource = from_str(yaml).unwrap();
        let mut expected_headers = std::collections::HashMap::new();
        expected_headers.insert(
            "Router-Authorization".to_string(),
            "${env.HELLO_TESTS}".to_string(),
        );
        let expected = SchemaSource::SubgraphIntrospection {
            subgraph_url: "https://example.com/graphql".parse().unwrap(),
            introspection_headers: Some(expected_headers),
        };
        assert_eq!(source, expected);
    }

    #[test]
    fn test_subgraph() {
        let yaml = r#"
graphref: my-graph@current
subgraph: my-subgraph
        "#;
        let source: SchemaSource = from_str(yaml).unwrap();
        let expected = SchemaSource::Subgraph {
            graphref: "my-graph@current".to_string(),
            subgraph: "my-subgraph".to_string(),
        };
        assert_eq!(source, expected);
    }

    #[test]
    fn test_sdl() {
        let yaml = r#"
sdl: |
    type Query {
        hello: String
    }"#;
        let source: SchemaSource = from_str(yaml).unwrap();
        let expected = SchemaSource::Sdl {
            sdl: "type Query {\n    hello: String\n}".to_string(),
        };
        assert_eq!(source, expected);
    }
}
