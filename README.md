# `federation-rs`

This repository is responsible for all of the [deno](https://deno.land)-powered TypeScript <--> Rust interop needed for the [`harmonizer`](https://crates.io/crates/harmonizer) crate.

## `harmonizer`

The `harmonizer` crate is a library that provides the federation composition algorithm to the rest of Apollo's Rust ecosystem.

### Branch Strategy

This repository has two long-running branches, much like the [`federation`](https://github.com/apollographql/federation) repo itself: `version-0.x` for Federation 1 and `main` for Federation 2.

### Publish Strategy

When a new version of `@apollo/composition` is published, Renovate opens a PR against `main` that bumps the dependency, and automatically merges it. Then a CircleCI job requests approval in Slack for cutting a release of `harmonizer`, and then builds and publishes `harmonizer` to crates.io.

When a new version of `@apollo/federation` is published, Renovate opens a PR against `version-0.x` that bumps the dependency, and automatically merges it. Then a CircleCI job requests approval in Slack for cutting a release of `harmonizer`, and then builds and publishes `harmonizer` to crates.io.

## `apollo-federation-types` and `apollo-supergraph-config`

The `apollo-federation-types` and `apollo-supergraph-config` crates are helper crates that are used across Apollo's federation ecosystem (primarily our [blazing-fast](https://www.apollographql.com/blog/announcement/backend/apollo-router-our-graphql-federation-runtime-in-rust/) [router](https://github.com/apollographql/router) and [rover](https://github.com/apollographql/rover)).

### Branch Strategy

These helper crates are only tracked on the `main` branch. `version-0.x` of `harmonizer` only relies on the published versions of these crates.

### Publish Strategy

To release new versions of the helper crates, create a new tag prefixed with the name of the crate, followed by the version, separated by a backslash. For example, if I was releasing `v1.0.0` of `apollo-federation-types`, I would run `git tag -a apollo-federation-types/v1.0.0` and `git push --tags`. This would kick off a release of the `apollo-federation-types` repository and nothing else.
