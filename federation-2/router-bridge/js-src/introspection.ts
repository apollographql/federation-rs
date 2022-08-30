import {
  buildSchema as gqlBuildSchema,
  ExecutionResult,
  GraphQLError,
  GraphQLSchema,
  graphqlSync,
} from "graphql";

import { buildSchema } from "@apollo/federation-internals";
import { QueryPlannerConfig } from "@apollo/query-planner";

export function batchIntrospect(
  sdl: string,
  queries: string[],
  options: QueryPlannerConfig
): ExecutionResult[] {
  let schema: GraphQLSchema;
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
  options: QueryPlannerConfig
): ExecutionResult {
  let schema: GraphQLSchema;
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
