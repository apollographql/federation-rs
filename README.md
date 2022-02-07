# `federation-rs`

This repository is responsible for all of the [deno](https://deno.land)-powered TypeScript <--> Rust interop needed for the [`harmonizer`](https://crates.io/crates/harmonizer) crate.

## Branch Strategy

This repository has one long-running branch, `main`. The [`federation`](https://github.com/apollographql/federation) repository itself maintains two separate branches, `version-0.x` for Federation 1 and `main` for Federation 2.

## `harmonizer`

The `harmonizer` crate is a library that provides the federation composition algorithm to the rest of Apollo's Rust ecosystem.

### `harmonizer-0` and `harmonizer-2`

You'll realize that there are two workspace crates for `harmonizer` in this repository: `harmonizer-0` and `harmonizer-2`. These will both be built by default when working in this repository, and they both use the same `harmonizer_build.rs` file to build. Since these two versions are both published as the [`harmonizer`](https://crates.io/crates/harmonizer) crate, `harmonizer_build.rs` takes care of updating the versions in the `Cargo.toml` and `package.json` files to match the corresponding JavaScript package (`@apollo/federation` for `harmonizer-0` and `@apollo/composition` for `harmonizer-2`). It then creates a `Cargo.publish.toml` that is _almost_ identical to the real `Cargo.toml` except it changes `package.name` from `harmonizer-x` to `harmonizer` and changes `package.publish` from `false` to `true`.

### Releasing `harmonizer`

When a new version of `@apollo/composition` is published, Renovate opens a PR against `main` that bumps the dependency in `harmonizer-2`, and automatically merges it. Then a CircleCI job requests approval in Slack for cutting a release of `harmonizer`, which, when approved, tags, builds, and publishes `harmonizer` to crates.io at the proper 2.x version.

When a new version of `@apollo/federation` is published, Renovate opens a PR against `main` that bumps the dependency in `harmonizer-0`, and automatically merges it. Then a CircleCI job requests approval in Slack for cutting a release of `harmonizer`, which, when approved, tags, builds, and publishes `harmonizer` to crates.io at the proper 0.x version.

## `supergraph`

The `supergraph` crate is a binary that provides the federation composition algorithm as a CLI, primarily for integration with [rover](https://github.com/apollographql/rover).

### `supergraph-0` and `supergraph-2`

Much like `harmonizer`, there are two sibling versions of `supergraph`. This works exactly the same as `harmonizer` except that `supergraph` is never published to crates.io. Their version numbers are updated when `harmonizer` is built, and only one version is selected when creating the `stage` workspace.

### Releasing `supergraph`

When `harmonizer` is published to crates.io, the `supergraph` binary is built for multiple architectures and uploaded to this repository's GitHub Releases page.

## `apollo-federation-types`

The `apollo-federation-types` crate is a helper crate that is used across Apollo's federation ecosystem (primarily in our [blazing-fast](https://www.apollographql.com/blog/announcement/backend/apollo-router-our-graphql-federation-runtime-in-rust/) [router](https://github.com/apollographql/router), [rover](https://github.com/apollographql/rover)), in both versions of `harmonizer` and their respective `supergraph` CLI.

### Publish Strategy

The helper crates are only ever released when a new version of harmonizer is published. xtask manages the creation of a "/stage" directory that only contains one version of harmonizer, then all of the crates are published together.
