---
name: release
description: Release new versions of apollo-federation-types and apollo-composition crates. Use when the developer asks to prepare a release, bump versions, or publish crates.
disable-model-invocation: true
argument-hint: [apollo-federation-version]
---

# Release Process for federation-rs

This skill guides the release of `apollo-federation-types` and `apollo-composition` crates to crates.io, triggered by pushing git tags that match the CircleCI release filter.

## Prerequisites

The developer must provide the target `apollo-federation` crate version (e.g., `2.11.0` or `2.12.0-preview.1`). This is the version of the `apollo-federation` Rust crate published from the Router repository.

## Current State

Gather current state before making changes:

```
Workspace dep:       !`grep 'apollo-federation' Cargo.toml | head -1`
federation-types:    !`grep '^version' apollo-federation-types/Cargo.toml`
composition:         !`grep '^version' apollo-composition/Cargo.toml`
composition dep:     !`grep 'apollo-federation-types' apollo-composition/Cargo.toml`
Current branch:      !`git branch --show-current`
```

## Determine Release Type

Ask the developer if unclear. The target `apollo-federation` version determines the release type:

- **Full release** (e.g., `2.11.0`) → target branch is `main`, strip all prerelease suffixes
- **Prerelease** (e.g., `2.12.0-preview.1`, `2.11.0-abstract.1`) → target branch is `next`, use matching prerelease suffixes

---

## Full Release (targeting `main`)

### 1. Verify the target apollo-federation version is published

Run `cargo search apollo-federation --limit 1` to confirm the target version exists on crates.io.

### 2. Create a release branch from main

```bash
git fetch origin
git checkout -b release/<descriptive-name> origin/main
```

If there is a `next` branch with unreleased work, merge it:

```bash
git merge origin/next --no-edit
```

### 3. Update workspace dependency

In `Cargo.toml` (workspace root), update to the final version (no prerelease suffix):
```toml
apollo-federation = "<target-version>"
```

### 4. Bump crate versions

Strip prerelease suffixes to produce final versions:

- `apollo-federation-types/Cargo.toml` — update `version` (e.g., `0.17.0-abstract.1` → `0.17.0`)
- `apollo-composition/Cargo.toml` — update `version` AND the `apollo-federation-types` dependency version

### 5. Update changelogs

Add new version entries to both:
- `apollo-federation-types/CHANGELOG.md`
- `apollo-composition/CHANGELOG.md`

Each entry should note the `apollo-federation` version update and any other changes included (check `git log origin/main..HEAD` for commits being merged).

Remove any stray prerelease changelog entries that may have been added on the `next` branch.

### 6. Verify build and tests

```bash
cargo check
cargo test
```

Both must pass before proceeding.

### 7. Commit

```
Release apollo-federation-types@v<version> and apollo-composition@v<version>

Update apollo-federation dependency to v<target-version>.

Changes:
- apollo-federation-types: <old> → <new>
- apollo-composition: <old> → <new>
- workspace apollo-federation dep: <old> → <new>
```

### 8. Push branch and create PR targeting `main`

Push the release branch (no tags yet) and create a PR.

### 9. Create and push tags sequentially (after PR review)

Tags must be pushed **one at a time** because `apollo-composition` depends on `apollo-federation-types` — the types crate must be published to crates.io before composition can be published.

Only after the PR is reviewed/approved:

**First**, create and push the `apollo-federation-types` tag:

```bash
git tag -a apollo-federation-types@v<version> -m "apollo-federation-types@v<version>"
git push origin apollo-federation-types@v<version>
```

Wait for CircleCI to successfully publish `apollo-federation-types` to crates.io.

**Then**, create and push the `apollo-composition` tag:

```bash
git tag -a apollo-composition@v<version> -m "apollo-composition@v<version>"
git push origin apollo-composition@v<version>
```

**Do not push any tags until the developer explicitly approves.**

---

## Prerelease (targeting `next`)

### 1. Verify the target apollo-federation prerelease version is published

Run `cargo search apollo-federation --limit 1` or check crates.io to confirm the prerelease version exists.

### 2. Create a release branch from next

```bash
git fetch origin
git checkout -b release/<descriptive-name> origin/next
```

### 3. Update workspace dependency

In `Cargo.toml` (workspace root), update to the prerelease version:
```toml
apollo-federation = "<target-prerelease-version>"
```

### 4. Bump crate versions with prerelease suffix

Use the same prerelease suffix as the `apollo-federation` version:

- `apollo-federation-types/Cargo.toml` — e.g., `0.17.0-preview.1`
- `apollo-composition/Cargo.toml` — version and `apollo-federation-types` dep

### 5. Update changelogs

Add prerelease version entries noting the `apollo-federation` dependency update.

### 6. Verify build and tests

```bash
cargo check
cargo test
```

### 7. Commit and push to `next`

Commit directly to `next` or create a PR targeting `next`.

### 8. Create and push tags sequentially

Same sequential process as full releases — push `apollo-federation-types` first, wait for crates.io publish, then push `apollo-composition`:

```bash
git tag -a apollo-federation-types@v<version> -m "apollo-federation-types@v<version>"
git push origin apollo-federation-types@v<version>
# Wait for CI to publish to crates.io...

git tag -a apollo-composition@v<version> -m "apollo-composition@v<version>"
git push origin apollo-composition@v<version>
```

**Do not push tags until the developer explicitly approves.**

---

## CI Release Trigger

Pushing tags matching `(apollo-federation-types@v.*)|(apollo-composition@v.*)` triggers the CircleCI release workflow which:
1. Runs tests on amd_ubuntu, arm_ubuntu, and arm_macos
2. If all pass, publishes to crates.io via `cargo publish`

## Important Notes

- **Tag format**: `package-name@vX.Y.Z` or `package-name@vX.Y.Z-suffix.N` (annotated tags, message = tag name)
- **Stable releases** merge to `main`; prereleases go to `next`
- **Never push tags** until the developer explicitly says to, as this can be a consequential action
