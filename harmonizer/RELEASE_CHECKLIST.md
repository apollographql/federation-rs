# Release Checklist

This is a list of the things that need to happen when publishing `harmonizer-2` and `supergraph-2` at the same time.

## Build a Release

### Changelog

None of the `federation-rs` packages currently maintain changelogs as they are largely mirrors of upstream packages. You're off the hook!

### Start a release PR

1. Make sure you have both `npm` and `cargo` installed on your machine and in your `PATH`.
1. Run `HARMONIZER_RELEASE_VERSION=composition@v{version}` where `{version}` is the new version you're bumping to. The major version should NOT be 0, it should be 2.
1. Run `git checkout main && git stash && git pull && git checkout -b $HARMONIZER_RELEASE_VERSION`.
1. Update the version of `@apollo/composition` in `./federation-2/harmonizer/package.json`
1. Run `cargo xtask dist --debug` from the root of `federation-rs`
1. Push up a commit containing the version bumps with the message `release: $HARMONIZER_RELEASE_VERSION`
1. Wait for tests to pass on the PR
1. Merge your PR to `main`

### Build and tag release

1. Once merged, run `git checkout main && git pull`
1. Run `cargo xtask tag --package $HARMONIZER_RELEASE_VERSION --real-publish`
1. Wait for CI to build and publish `harmonizer` to crates.io and `supergraph` to `federation-rs` GitHub releases.

### Update the latest version delivered by Rover

In order to update the latest version delivered by Rover, you will need to submit a PR against the `main` branch that bumps the appropriate version in `./latest_plugin_versions.json`.

## Troubleshooting a release

Mistakes happen. Most of these release steps are recoverable if you mess up.

### I pushed the wrong tag

Tags and releases can be removed in GitHub. First, [remove the remote tag](https://stackoverflow.com/questions/5480258/how-to-delete-a-remote-tag):

```console
git push --delete origin $HARMONIZER_RELEASE_VERSION
```

This will turn the release into a `draft` and you can delete it from the edit page.

Make sure you also delete the local tag:

```console
git tag --delete $HARMONIZER_RELEASE_VERSION
```

### I got an error from cargo complaining about the version of `apollo-federation-types`

This likely means that the version has been bumped in `apollo-federation-types` and it hasn't been published yet. You'll need to follow the steps in that [release checklist](../apollo-federation-types/RELEASE_CHECKLIST.md) prior to publishing.
