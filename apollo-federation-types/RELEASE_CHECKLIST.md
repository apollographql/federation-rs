# Release Checklist

This is a list of the things that need to happen when publishing `apollo-federation-types`.

## Build a Release

### Create and merge your release PR

1. Create a branch
2. Update the `CHANGELOG.md` file in this directory. This is done completely by hand today.
3. Update the version of `apollo-federation-types` in `Cargo.toml`
4. Push up a commit and open a PR to `main`
5. Wait for tests to pass on the PR, then merge to `main`

### Build and tag release

1. Once merged, run `git switch main && git pull`
2. Create and push a tag called `apollo-federation-types@v.<version>` where `<version>` is the version you just updated
   in `Cargo.toml`
3. Wait for CI to build and publish `apollo-federation-types` to crates.io.
