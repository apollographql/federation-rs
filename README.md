# `federation-rs`

This repository is responsible for all of the [deno](https://deno.land)-powered TypeScript <--> Rust interop. Currently
this includes composition and query planning.

## Branch Strategy

This repository has one long-running branch, `main`. The [`federation`](https://github.com/apollographql/federation)
repository itself maintains two separate branches, `version-0.x` for Federation 1 and `main` for Federation 2.

## `federation-rs` Crates

Each crate listed here has their own README with much more information than what's here.

### `harmonizer`

**The `harmonizer` crate is a library that provides the federation composition algorithm to the rest of Apollo's Rust
ecosystem.**

### `supergraph`

**The `supergraph` crate is a binary that provides the federation composition algorithm as a CLI, primarily for
integration with [rover](https://github.com/apollographql/rover).**

### `apollo-federation-types`

The `apollo-federation-types` crate provides types for all versions of `harmonizer` and `supergraph`, and is used
by [Rover](https://github.com/apollographql/rover) to read the output from the `supergraph` binary.

### `router-bridge`

**The `router-bridge` crate is a library that provides the federation query-planning algorithm, primarily for
integration with the [router](https://github.com/apollographql/router)**
