# Changelog

Not every version is listed here because versions before 0.14.0 did not have a changelog.

## 0.15.2

### Features

- Prepend `[subgraph_name]` to Issue messages for better error attribution.

## 0.15.1

### Features

- Added new `composition` module behind the `composition` Cargo feature for types related to composition (previously in the `apollo-composition` crate).

## 0.15.0

### Breaking changes

- `GraphQLError.nodes` is now an `Option<Vec<SubgraphASTNode>>`
- All usages of `camino::Utf8PathBuf` have been replaced with `std::path::PathBuf`

### Features

- A new `json_schema` feature derives the `schemars::JsonSchema` trait on `SupergraphConfig` and its sub-types.

## 0.14.1 - 2024-09-19

### Features

- `impl FromIterator<(String, SubgraphConfig)> for SupergraphConfig`

## 0.14.0 - 2024-09-11

### Breaking changes

- Removed `BuildErrorNode` in favor of `BuildMessageLocation`.
- Removed `BuildErrorNodeLocationToken`
- `BuildMessagePoint` now uses `usize` instead of `u32`
- The `build` mod has been renamed `rover` to better represent the interface.
- `SubgraphDefinition` is in the new `javascript` mod.
- Removed `SubgraphDefinition::new` which was just a cloning wrapper around `pub` attributes

### Features

- `impl From<BuildMessage> for BuildHint`
- `impl From<BuildMessage> for BuildError`
- `impl From<PluginResult> for BuildResult`
- Added a new `javascript` mod for types matching the `@apollo/composition` JavaScript package.
