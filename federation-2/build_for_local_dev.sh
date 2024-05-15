#!/usr/bin/env bash
set -euo pipefail
trap 'echo "ERR (rc: $?)"' ERR

# Creates a new `supergraph` binary for use in local development and copies it to the local ~/.rover/bin directory for
# end-to-end testing with `rover` and Apollo Workbench.

# Get the path to the local federation repo
if [ -z "${1:-}" ]; then
  echo "Usage: $0 <path-to-federation-repo>"
  exit 1
fi
FEDERATION_JS_PATH="$1"

# Stage 1: Build a local copy of all the JS federation stuff
pushd "$FEDERATION_JS_PATH"

npm run compile

pushd internals-js
INTERNALS_TARBALL=$(npm pack | tail -n 1)
VERSION=$(echo "$INTERNALS_TARBALL" | grep -oE '\d+\.\d+\.\d+(-\w+\.\d+)?')
STABLE_COMPONENT=$(echo "$VERSION" | cut -d'-' -f1)
popd

pushd query-graphs-js
# Use the packed version of internals
npm install "$FEDERATION_JS_PATH"/internals-js/"$INTERNALS_TARBALL"
QUERY_GRAPHS_TARBALL=$(npm pack | tail -n 1)
# Restore original install
npm i -E @apollo/federation-internals@"$VERSION"
popd

pushd composition-js
# Use the packed version of internals and query-graphs
npm install "$FEDERATION_JS_PATH"/internals-js/"$INTERNALS_TARBALL"
npm install "$FEDERATION_JS_PATH"/query-graphs-js/"$QUERY_GRAPHS_TARBALL"
COMPOSITION_TARBALL=$(npm pack | tail -n 1)
# Restore original install
npm i -E @apollo/federation-internals@"$VERSION"
npm i -E @apollo/query-graphs@="$VERSION"
popd

popd

# Stage 2: Build a local copy of the Rust federation stuff

# If jq or jaq is installed, capture the current version of federation to restore later
if command -v jq &> /dev/null; then
  CURRENT_VERSION=$(jq -r '.dependencies."@apollo/composition"' harmonizer/package.json)
elif command -v jaq &> /dev/null; then
  CURRENT_VERSION=$(jaq -r '.dependencies."@apollo/composition"' harmonizer/package.json)
fi

npm i --prefix harmonizer "$FEDERATION_JS_PATH"/composition-js/"$COMPOSITION_TARBALL"
SKIP_MANIFESTS=true cargo build --package supergraph

# Stage 3: Copy the binary to the local rover directory
cp target/debug/supergraph ~/.rover/bin/supergraph-v"$STABLE_COMPONENT"

# Stage 4: Clean up
rm "$FEDERATION_JS_PATH"/internals-js/"$INTERNALS_TARBALL"
rm "$FEDERATION_JS_PATH"/query-graphs-js/"$QUERY_GRAPHS_TARBALL"
rm "$FEDERATION_JS_PATH"/composition-js/"$COMPOSITION_TARBALL"
[[ -n "$CURRENT_VERSION" ]] && npm i -E --prefix harmonizer "@apollo/composition@$CURRENT_VERSION"
