# Release Checklist

This is a list of the things that need to happen when publishing `router-bridge`.

## Build a Release

### Changelog

None of the `federation-rs` packages currently maintain changelogs as they are largely mirrors of upstream packages. You're off the hook!

### Start a release PR

1. Make sure you have both `npm` and `cargo` installed on your machine and in your `PATH`.
1. Run `ROUTERBRIDGE_RELEASE_VERSION=router-bridge@v{version}` where `{version}` is the new version you're bumping to.
1. Run `git checkout main && git stash && git pull && git checkout -b $ROUTERBRIDGE_RELEASE_VERSION`.
1. Update the version of `@apollo/federation-internals` and `@apollo/query-planner` in the `package.json`.
1. Then run `npm install` from the `federation-2/router-bridge` directory to let it update the `package-lock.json`.
1. Update the version of `router-bridge` in `Cargo.toml`
1. Run `cargo build -p router-bridge` from the `federation-2/` workspace
1. Push up a commit containing the version bumps with the message `release: $ROUTERBRIDGE_RELEASE_VERSION`
1. Wait for tests to pass on the PR
1. Merge your PR to `main`

### Build and tag release

1. Once merged, run `git checkout main && git pull`
1. Return to the root of the repository if you're not already there.
1. Run `cargo xtask tag --package $ROUTERBRIDGE_RELEASE_VERSION --real-publish`
1. Wait for CI to build and publish `router-bridge` to crates.io.

## Troubleshooting a release

Mistakes happen. Most of these release steps are recoverable if you mess up.

### I pushed the wrong tag

Tags and releases can be removed in GitHub. First, [remove the remote tag](https://stackoverflow.com/questions/5480258/how-to-delete-a-remote-tag):

```console
git push --delete origin $ROUTERBRIDGE_RELEASE_VERSION
```

This will turn the release into a `draft` and you can delete it from the edit page.

Make sure you also delete the local tag:

```console
git tag --delete $ROUTERBRIDGE_RELEASE_VERSION
```
