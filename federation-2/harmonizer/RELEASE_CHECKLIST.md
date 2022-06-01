# Release Checklist

This is a list of the things that need to happen when publishing `harmonizer-2` and `supergraph-2` at the same time.

## Build a Release

### Changelog

None of the `federation-rs` packages currently maintain changelogs as they are largely mirrors of upstream packages. You're off the hook!

### Start a release PR

1. Make sure you have both `npm` and `cargo` installed on your machine and in your `PATH`.
1. Run `PUBSLUG=composition@v{version}` where `{version}` is the new version you're bumping to. The major version should NOT be 0, it should be 2.
1. Run `git checkout main && git stash && git pull && git checkout -b $PUBSLUG`.
1. Update the version of `@apollo/composition` in `./harmonizer-2/package.json`
1. Run `cargo xtask dist --debug` from the root of `federation-rs`
1. Push up a commit containing the version bumps with the message `release: $PUBSLUG`
1. Wait for tests to pass on the PR
1. Merge your PR to `main`

### Build and tag release

1. Once merged, run `git checkout main && git pull`
1. Run `cargo xtask tag --package $PUBSLUG --real-publish`
   - **NOTE** If you get an error, you might need to run `git fetch --tags -f` to ensure the remote tags match the local tags
1. Wait for CI to build and publish `harmonizer` to crates.io and `supergraph` to `federation-rs` GitHub releases.

### Tag the `latest-2` release so rover automatically downloads the new version

1. Run `git tag -d composition-latest-2 && git tag -a composition-latest-2 -m v{version} && git push --tags -f` replacing `{version}` with the version of composition.

TODO: tag latest-2 / latest-1

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

### I got an error from cargo complaining about the version of `apollo-federation-types`

This likely means that the version has been bumped in `apollo-federation-types` and it hasn't been published yet. You'll need to follow the steps in that [release checklist](../apollo-federation-types/RELEASE_CHECKLIST.md) prior to publishing.
