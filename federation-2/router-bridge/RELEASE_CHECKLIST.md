# Release Checklist

This is a list of the things that need to happen when publishing `router-bridge`.

## Build a Release

### Changelog

None of the `federation-rs` packages currently maintain changelogs as they are largely mirrors of upstream packages. You're off the hook!

### ~~Start a release PR~~ This should be handled by the "Release Components" Github Action, but instructions are preserved in case it needs to be done manually.

~~1. Make sure you have both `npm` and `cargo` installed on your machine and in your `PATH`.~~
~~2. Run `ROUTERBRIDGE_RELEASE_VERSION=router-bridge@v{version}` where `{version}` is the new version you're bumping to.~~
~~3. Run `git checkout main && git stash && git pull && git checkout -b $ROUTERBRIDGE_RELEASE_VERSION`.~~
~~4. Update the version of `@apollo/federation-internals` and `@apollo/query-planner` in the `package.json`.~~
~~5. Then run `npm install` from the `federation-2/router-bridge` directory to let it update the `package-lock.json`.~~
~~6. Update the version of `router-bridge` in `Cargo.toml`~~
~~7. Run `cargo build -p router-bridge` from the `federation-2/` workspace~~
~~8. Push up a commit containing the version bumps with the message `release: $ROUTERBRIDGE_RELEASE_VERSION`~~
~~9. Wait for tests to pass on the PR~~
~~10. Merge your PR to `main`~~

### ~~Build and tag release~~ This should be handled by the "Publish Components" Github Action, but instructions are preserved in case it needs to be done manually.

~~1. Once merged, run `git checkout main && git pull`~~
~~2. Return to the root of the repository if you're not already there.~~
~~3. Run `cargo xtask tag --package $ROUTERBRIDGE_RELEASE_VERSION --real-publish`~~
~~4. Wait for CI to build and publish `router-bridge` to crates.io.~~

### Releasing
1. When `federation` is released, it should run the "Release Components" Github Action which will create a PR for the `harmonizer` and `router-bridge`. Identify the correct PR and validate that everything looks good. If for some reason the Github action needs to be run manually, it can be done so from https://github.com/apollographql/federation-rs/actions/workflows/release.yml (you will need to pass in the version you are releasing as an input).
1. If the PR build doesn't pass, you may need to make updates the code in order to get it working.
1. Once everything builds correctly, approve and merge the PR.
1. Validate that the "Publish Components" action is created and runs correctly.


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
