import {
  buildSchema as buildGraphqlSchema,
  ExecutionResult,
  GraphQLError,
  printSchema,
} from "graphql";

import { buildSupergraphSchema } from "@apollo/federation-internals";

export function apiSchema(sdl: string): ExecutionResult<String> {
  let schema: String;
  try {
    // First go through regular schema parsing
    buildGraphqlSchema(sdl);

    // Now try to get the API schema
    let [composedSchema] = buildSupergraphSchema(sdl);

    let apiSchema = composedSchema.toAPISchema();
    schema = printSchema(apiSchema.toGraphQLJSSchema());
  } catch (e) {
    return {
      errors: [e],
    };
  }
  if (!schema) {
    return {
      errors: [new GraphQLError("couldn't build api schema from SDL")],
    };
  }
  return { data: schema, errors: [] };
}
