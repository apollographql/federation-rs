# Changelog

## 0.2.6

- Update to `apollo-federation-types` 0.15.3

## 0.2.5

- Update to `apollo-federation` 2.0.0

## 0.2.2

- Prepend `[subgraph_name]` to Issue messages for better error attribution.

## 0.2.0

- Pin `apollo-federation` to 2.0.0-preview.4 to prevent future breaking changes
- Move `Issue`, `Severity`, and `SubgraphLocation` to new `apollo_federation_types::composition` module so some
  consumers can avoid pulling in extra dependencies. Requires `apollo_federation_types`

## 0.1.6

- Update to `apollo-federation` 2.0.0-preview.3

## 0.1.5

- [#590](https://github.com/apollographql/federation-rs/pull/590) Fix
  deserialization of `GraphQLError` nodes.
- Update to `apollo-federation` 2.0.0-preview.1

## 0.1.4

- Update to `apollo-federation` 2.0.0-preview.0

## 0.1.3

- [#586](https://github.com/apollographql/federation-rs/pull/586) Make
  `SubgraphLocation.subgraph` an `Option`. For now, composition errors can have
  no attributed subgraph.
- [#583](https://github.com/apollographql/federation-rs/pull/583) Remove
  connectors warning.

## 0.1.2

- Updated dependencies
