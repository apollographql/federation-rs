# Composition

This crate orchestrates composition between the TypeScript `@apollo/composition` and the Rust `apollo-federation`.

Consumers are expected to bring their own implementation of the TypeScript component. `harmonizer` in this
workspace is an example of such an implementation using Deno to execute the TypeScript code.
