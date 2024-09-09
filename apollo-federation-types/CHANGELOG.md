# Changelog

Not every version is listed here because versions before 0.14.0 did not have a changelog.

## 0.14.0 - 2024-09-09

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
