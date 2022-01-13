# `federation-rs`

This repository is responsible for all of the Rust <--> JavaScript interop.

## Implementation

This repository uses git submodules to handle sourcing the JavaScript that will be embedded with deno. To get started, clone the repo with the following invocation: `git clone --recurse-submodules https://github.com/apollographql/federation-rs`.

This repository has two long-running branches, much like the [`federation`](https://github.com/apollographql/federation) repo itself: `version-0.x` for Federation 1 and `main` for Federation 2. The `federation-js` submodule should be pointed at the commit on the mirrored branch at all times, this is handled automatically with TODO.

## Release Information

Any time a new release is made from the `federation` repo, Renovate will open a PR updating the dependencies, and a release of the new Rust libraries will happen automatically.
