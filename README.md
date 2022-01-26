# `federation-rs`

This repository is responsible for all of the [deno](https://deno.land)-powered TypeScript <--> Rust interop needed for the [`harmonizer`](https://crates.io/crates/harmonizer) crate.

## Branch Strategy

This repository has one long-running branch, `main`. The [`federation`](https://github.com/apollographql/federation) repository itself maintains two separate branches, `version-0.x` for Federation 1 and `main` for Federation 2.

## `harmonizer`

The `harmonizer` crate is a library that provides the federation composition algorithm to the rest of Apollo's Rust ecosystem.

### Releasing `harmonizer`

When a new version of `@apollo/composition` is published, Renovate opens a PR against `main` that bumps the dependency in `harmonizer-2`, and automatically merges it. Then a CircleCI job requests approval in Slack for cutting a release of `harmonizer`, which, when approved, tags, builds, and publishes `harmonizer` to crates.io at the proper 2.x version.

When a new version of `@apollo/federation` is published, Renovate opens a PR against `main` that bumps the dependency in `harmonizer-0`, and automatically merges it. Then a CircleCI job requests approval in Slack for cutting a release of `harmonizer`, which, when approved, tags, builds, and publishes `harmonizer` to crates.io at the proper 0.x version.

## `apollo-federation-types` and `apollo-supergraph-config`

The `apollo-federation-types` and `apollo-supergraph-config` crates are helper crates that are used across Apollo's federation ecosystem (primarily our [blazing-fast](https://www.apollographql.com/blog/announcement/backend/apollo-router-our-graphql-federation-runtime-in-rust/) [router](https://github.com/apollographql/router), [rover](https://github.com/apollographql/rover)), and in both versions of harmonizer.

### Publish Strategy

To release new versions of the helper crates, create a new tag prefixed with the name of the crate, followed by the version, separated by a backslash. For example, if I was releasing `v1.0.0` of `apollo-federation-types`, I would run `git tag -a apollo-federation-types/v1.0.0` and `git push --tags`. This would kick off a release of the `apollo-federation-types` repository and nothing else.
