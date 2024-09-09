# `federation-rs`

This repository is responsible for all the TypeScript <--> Rust interop. Currently
this includes composition and query planning.

## Branch Strategy

`main` is for the latest stable federation v2.x release. We can create support branches for older versions of federation
(like `support/v1`).

## Crates

Each crate listed here has their own README with much more information than what's here.

### `apollo-composition`

Bridges the gap between the JavaScript [federation](https://github.com/apollographql/federation) and the Rust
[apollo-federation](https://github.com/apollographql/router) libraries for composition.

### `apollo-federation-types`

The `apollo-federation-types` crate has shared types used for both Rover and Apollo GraphOS services, primarily
around the composition process.

### `router-bridge`

**The `router-bridge` crate is a library that provides the federation query-planning algorithm, primarily for
integration with the [router](https://github.com/apollographql/router)**
