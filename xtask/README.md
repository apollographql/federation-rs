# xtask

We use [xtask](https://github.com/matklad/cargo-xtask) for automating our tests, lints, and releases.

You can run `cargo xtask --help` to see the usage. Generally we recommend that you continue to use the default cargo commands like `cargo fmt`, `cargo clippy`, and `cargo test`, from one of the workspace roots, but it can be helpful to use xtask to run all the tests all at once.

You'll note that this repository is broken up into three Cargo workspaces. One at the root that contains `xtask` and `apollo-federation-types`. Then there are two other Cargo workspaces in `federation-1` and `federation-2` that both refer to `apollo-federation-types`.

Xtask uses a type called `PackageTag` that takes a package group and a version and maps it to the correct workspace directory. The package tags look like this:

`composition@v2.0.0` refers to a `PackageGroup` containing two packages, `harmonizer@v2.0.0` and `supergraph@v2.0.0`, each with source code located in `./federation-2`.

`composition@v0.35.3` refers to `harmonizer@v0.35.3` and `supergraph@v0.35.3`, each with source code located in `./federation-1`.

Most xtask commands used for publishing require that you specify a package tag for a specific package. Xtask commands used locally for debugging will allow you to specify one, but you may skip specifying it altogether if you would like to run that command across _all_ workspaces.

Unfortunately, running `cargo test`/`cargo build` by itself in the root of `federation-rs` will only run tests for `apollo-federation-types` and `xtask`, it won't run them for the mission-critical workspace crates containing source for the public-facing libraries/binaries.

## Important Commands

Ordered from most local to most CI.

## xtask tag

You can run `cargo xtask tag` to kick off a release. Each package group has their own release checklist and explains exactly how to run this command.

## xtask test

You can run `cargo xtask test` to run tests across all of the workspace crates.

You can limit it to only run on a specific workspace by specifying a package tag by running `cargo xtask test --package composition@v2.0.0`

## xtask dist

You can run `cargo xtask dist` to build all of the workspace crates. You probably want to specify the `--debug` flag when running locally to speed things up.

You can limit it to only run on a specific workspace by specifying a package tag by running `cargo xtask dist --package composition@v2.0.0`

Note that it will output to the `target` directory in the workspace itself, not necessarily in the root.

That being said, you should never really need to run this command if you're using rust-analyzer and the VS Code workspace defined in `main.code-workspace`.

## xtask package

You can run `cargo xtask package --package composition@v2.0.0` to create tarballs for a specific version of a package group.

This is usually orchestrated by `xtask publish` and run in CI, but it can be helpful to run it locally to verify that things are working smoothly.

## xtask publish

You can run `cargo xtask package --package composition@v2.0.0` to publish binaries to a GitHub release and/or publish a crate to crates.io for a specific version of a package group.

This is usually kicked off in CI after `cargo xtask tag` has been run.
