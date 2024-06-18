import {
  buildSchema as gqlBuildSchema,
  ExecutionResult,
  GraphQLError,
  GraphQLSchema,
  graphqlSync,
} from "graphql";

import { buildSchema } from "@apollo/federation-internals";
import { QueryPlannerConfigExt } from "./types";

export function batchIntrospect(
  sdl: string,
  queries: string[],
  options: QueryPlannerConfigExt
): ExecutionResult[] {
  let schema: GraphQLSchema;

  const validate = options.graphqlValidation ?? true;
  if (validate) {
    try {
      // First go through regular schema parsing
      gqlBuildSchema(sdl);
    } catch (err) {
      return Array(queries.length).fill({
        errors: [Object.assign(err, { validationError: true })],
      });
    }
  }

  try {
    // First go through regular schema parsing
    gqlBuildSchema(sdl);

    // Now try to get the API schema
    let composedSchema = buildSchema(sdl);
    let apiSchema = composedSchema.toAPISchema();
    schema = apiSchema.toGraphQLJSSchema({
      includeDefer: options.incrementalDelivery?.enableDefer,
    });
  } catch (e) {
    return Array(queries.length).fill({
      errors: [e],
    });
  }
  if (!schema) {
    return Array(queries.length).fill({
      errors: [new Error(`couldn't build schema from SDL`)],
    });
  }
  return queries.map((query) => introspectOne(schema, query));
}

export function introspect(
  sdl: string,
  query: string,
  options: QueryPlannerConfigExt
): ExecutionResult {
  let schema: GraphQLSchema;

  const validate = options.graphqlValidation ?? true;
  if (validate) {
    try {
      // First go through regular schema parsing
      gqlBuildSchema(sdl);
    } catch (err) {
      return {
        errors: [Object.assign(err, { validationError: true })],
      };
    }
  }

  try {
    // Now try to get the API schema
    let composedSchema = buildSchema(sdl);
    let apiSchema = composedSchema.toAPISchema();
    schema = apiSchema.toGraphQLJSSchema({
      includeDefer: options.incrementalDelivery?.enableDefer,
    });
  } catch (e) {
    return {
      errors: [e],
    };
  }
  if (!schema) {
    return {
      errors: [new GraphQLError("couldn't build schema from SDL")],
    };
  }
  return introspectOne(schema, query);
}

const introspectOne = (
  schema: GraphQLSchema,
  query: string
): ExecutionResult => {
  const { data, errors } = graphqlSync({ schema, source: query });

  if (errors) {
    return { data, errors: [...errors] };
  } else {
    return { data, errors: [] };
  }
};
