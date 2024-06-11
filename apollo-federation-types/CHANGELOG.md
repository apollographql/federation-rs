# Changelog

Not every version is listed here because versions before 0.12.0 did not have a changelog.

## 0.12.0

### Breaking changes

- Removed `BuildErrorNode` in favor of `BuildMessageLocation`.
- Removed `BuildErrorNodeLocationToken`
- `BuildMessagePoint` now uses `usize` instead of `u32`

### Features

- `impl From<BuildMessage> for BuildHint`
- `impl From<BuildMessage> for BuildError`
- `impl From<PluginResult> for BuildResult`


