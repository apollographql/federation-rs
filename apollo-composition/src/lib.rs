use apollo_compiler::Schema;
use apollo_federation::sources::connect::{validate, GraphQLLocation, ValidationErrorCode};
use apollo_federation_types::build::{
    BuildError, BuildErrorNode, BuildErrorNodeLocationToken, BuildErrors, BuildResult,
    SubgraphDefinition,
};

/// Runs the complete composition process, hooking into both the Rust and JavaScript implementations.
///
/// TODO: Replace BuildResult with something more general-purpose (not Rover-specific at all)
pub fn compose<JavaScript: JavaScriptExecutor>(
    subgraph_definitions: Vec<SubgraphDefinition>,
) -> BuildResult {
    let subgraph_validation_errors: Vec<BuildError> = subgraph_definitions
        .iter()
        .flat_map(|subgraph| {
            // TODO: Use parse_and_validate (adding in directives as needed)
            // TODO: Handle schema errors rather than relying on JavaScript to catch it later
            let schema = Schema::parse(&subgraph.sdl, &subgraph.name)
                .unwrap_or_else(|schema_with_errors| schema_with_errors.partial);
            validate(schema).into_iter().map(|validation_error| {
                BuildError::composition_error(
                    Some(transform_code(validation_error.code)),
                    Some(validation_error.to_string()),
                    Some(transform_nodes(&subgraph.name, validation_error.locations)),
                    None, // TODO: Find out what omitted nodes are for
                )
            })
        })
        .collect();
    match JavaScript::compose(subgraph_definitions) {
        Ok(result) => {
            if !subgraph_validation_errors.is_empty() {
                Err(BuildErrors::from(subgraph_validation_errors))
            } else {
                // TODO: Rust-based supergraph validation
                Ok(result)
            }
        }
        Err(mut errors) => {
            for error in subgraph_validation_errors {
                errors.push(error);
            }
            Err(errors)
        }
    }
}

pub trait JavaScriptExecutor {
    fn compose(subgraph_definitions: Vec<SubgraphDefinition>) -> BuildResult;
}

fn transform_nodes(subgraph: &str, locations: Vec<GraphQLLocation>) -> Vec<BuildErrorNode> {
    locations
        .into_iter()
        .map(|location| BuildErrorNode {
            subgraph: Some(subgraph.to_string()),
            start: Some(BuildErrorNodeLocationToken {
                start: None,
                end: None,
                line: Some(location.start_line),
                column: Some(location.start_column),
            }),
            end: Some(BuildErrorNodeLocationToken {
                start: None,
                end: None,
                line: Some(location.end_line),
                column: Some(location.end_column),
            }),
            source: None,
        })
        .collect()
}

fn transform_code(code: ValidationErrorCode) -> String {
    match code {
        ValidationErrorCode::GraphQLError => "GRAPHQL_ERROR".to_string(),
        ValidationErrorCode::DuplicateSourceName => "DUPLICATE_SOURCE_NAME".to_string(),
        ValidationErrorCode::InvalidSourceName => "INVALID_SOURCE_NAME".to_string(),
        ValidationErrorCode::EmptySourceName => "EMPTY_SOURCE_NAME".to_string(),
        ValidationErrorCode::SourceUrl => "SOURCE_URL".to_string(),
        ValidationErrorCode::SourceScheme => "SOURCE_SCHEME".to_string(),
    }
}
