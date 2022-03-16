# `federation-rs`

This repository is responsible for all of the [deno](https://deno.land)-powered TypeScript <--> Rust interop. Currently this includes composition and query planning.

## Branch Strategy

This repository has one long-running branch, `main`. The [`federation`](https://github.com/apollographql/federation) repository itself maintains two separate branches, `version-0.x` for Federation 1 and `main` for Federation 2.

## Workspaces

You'll notice that there are three cargo workspaces in this repository. The first is the root which contains `xtask` and `apollo-federation-types`. The others are `federation-1` and `federation-2`. Each of these workspace directories contains some binaries and some libraries. Building specific packages is orchestrated by `xtask` in CI.

IMPORTANT: If you are working locally, changes made to `federation-1` and `federation-2` WILL NOT be picked up by `rust-analyzer` or your `cargo build`/`cargo test` commands. There are a few tools to help you out here.

1) You can run `code federation-rs.code-workspace` to open a VS Code Workspace that will run Rust-Analyzer properly.
1) You can run `cargo xtask test` from the root workspace directory to run tests across all workspaces
1) You can open a new VS Code window for `federation-1` and/or `federation-2` and `rust-analyzer` will work as expected. There might be some way to get something to work with [`rust-analyzer.linkedProjects`](https://rust-analyzer.github.io/manual.html) but it wouldn't be straightforward and opening a new VS Code window is easy enough.

## `federation-rs` Crates

Each crate listed here has their own README with much more information than what's here.

### `harmonizer`

**The `harmonizer` crate is a library that provides the federation composition algorithm to the rest of Apollo's Rust ecosystem.**

### `supergraph`

**The `supergraph` crate is a binary that provides the federation composition algorithm as a CLI, primarily for integration with [rover](https://github.com/apollographql/rover).**

### `apollo-federation-types`

The `apollo-federation-types` crate provides types for all versions of `harmonizer` and `supergraph`, and is used by [Rover](https://github.com/apollographql/rover) to read the output from the `supergraph` binary.

### `router-bridge`

**The `router-bridge` crate is a library that provides the federation query-planning algorithm, primarily for integration with the [router](https://github.com/apollographql/router)**
