# Release Checklist

This is a list of the things that need to happen when publishing `apollo-federation-types`.

## Build a Release

### Changelog

None of the `federation-rs` packages currently maintain changelogs as they are largely mirrors of upstream packages. You're off the hook!

### Create and merge your release PR

1. Make sure you have both `npm` and `cargo` installed on your machine and in your `PATH`.
1. Run `PUBSLUG=apollo-federation-types@v{version}` where `{version}` is the new version you're bumping to.
1. Run `git checkout main && git stash && git pull && git checkout -b $PUBSLUG`.
1. Update the version of `apollo-federation-types` in `Cargo.toml`
1. Update the versions of `apollo-federation-types` in `./federation-1/harmonizer/Cargo.toml` and `./federation-2/harmonizer/Cargo.toml`
1. Run `cargo xtask dist --debug` from the root of `federation-rs`
1. Push up a commit containing the version bumps with the message `release: $PUBSLUG`
1. Wait for tests to pass on the PR
1. Merge your PR to `main`

### Build and tag release

1. Once merged, run `git checkout main && git pull`
1.Run `cargo xtask tag --package $PUBSLUG --real-publish`
1. Wait for CI to build and publish `apollo-federation-types` to crates.io.

## Troubleshooting a release

Mistakes happen. Most of these release steps are recoverable if you mess up.

### I pushed the wrong tag

Tags and releases can be removed in GitHub. First, [remove the remote tag](https://stackoverflow.com/questions/5480258/how-to-delete-a-remote-tag):

```console
git push --delete origin $PUBSLUG
```

This will turn the release into a `draft` and you can delete it from the edit page.

Make sure you also delete the local tag:

```console
git tag --delete $PUBSLUG
```
