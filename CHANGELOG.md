# Changelog

This repository is a mirror of [`apollographql/federation`](http://github.com/apollographql/federation), providing a mechanism for Apollo's various Rust applications to take advantage of Apollo's JavaScript libraries. 

For user-facing changes to the underlying npm packages, please see the following changelogs:

|package|command/fn|version|changelog|npm package|
|--|--|--|--|--|
|`supergraph`/`harmonizer`|`compose`/`harmonize`|v1|[link](https://github.com/apollographql/federation/blob/version-0.x/federation-js/CHANGELOG.md)|[`@apollo/federation`](https://www.npmjs.com/package/@apollo/federation)|
|`supergraph`/`harmonizer`|`compose`/`harmonize`|v2|[link](https://github.com/apollographql/federation/blob/main/composition-js/CHANGELOG.md)|[`@apollo/composition`](https://www.npmjs.com/package/@apollo/composition)|
|`router-bridge`|`{N/A}`/`Planner::plan`|v2|[link](https://github.com/apollographql/federation/blob/main/query-planner-js/CHANGELOG.md)|[`@apollo/query-planner`](https://www.npmjs.com/package/@apollo/query-planner)|

---

We do not keep a changelog (as of now) for the Rust code that wraps these JavaScript packages. We try to do a good job with the [git history of this repository](https://github.com/apollographql/federation-rs/commits/main), and you can also search our [merged pull requests](https://github.com/apollographql/federation-rs/pulls?q=is%3Apr+is%3Amerged).
