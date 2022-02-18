# `federation-rs`

This repository is responsible for all of the [deno](https://deno.land)-powered TypeScript <--> Rust interop. Currently this includes composition and query planning.

## Branch Strategy

This repository has one long-running branch, `main`. The [`federation`](https://github.com/apollographql/federation) repository itself maintains two separate branches, `version-0.x` for Federation 1 and `main` for Federation 2.

## Packages

### `harmonizer`

**The `harmonizer` crate is a library that provides the federation composition algorithm to the rest of Apollo's Rust ecosystem.**

#### `harmonizer-0` and `harmonizer-2`

You'll realize that there are two workspace crates for `harmonizer` in this repository: `harmonizer-0` and `harmonizer-2`. These will both be built by default when working in this repository, and they both use the same `harmonizer_build.rs` file to build. Since these two versions are both published as the [`harmonizer`](https://crates.io/crates/harmonizer) crate, `harmonizer_build.rs` takes care of updating the versions in the `Cargo.toml` and `package.json` files to match the corresponding JavaScript package (`@apollo/federation` for `harmonizer-0` and `@apollo/composition` for `harmonizer-2`). It then creates a `Cargo.publish.toml` that is _almost_ identical to the real `Cargo.toml` except it changes `package.name` from `harmonizer-x` to `harmonizer` and changes `package.publish` from `false` to `true`.

### `supergraph`

**The `supergraph` crate is a binary that provides the federation composition algorithm as a CLI, primarily for integration with [rover](https://github.com/apollographql/rover).**

#### `supergraph-0` and `supergraph-2`

Much like `harmonizer`, there are two sibling versions of `supergraph`. This works exactly the same as `harmonizer` except that `supergraph` is never published to crates.io. Their version numbers are updated when `harmonizer` is built, and only one version is selected when creating the `stage` workspace.

### `apollo-federation-types`

The `apollo-federation-types` crate provides types for all versions of `harmonizer` and `supergraph`, and is used by [Rover](https://github.com/apollographql/rover) to read the output from the `supergraph` binary.
