import {
  buildSchema as buildGraphqlSchema,
  ExecutionResult,
  GraphQLError,
  printSchema,
} from "graphql";

import { buildSupergraphSchema } from "@apollo/federation-internals";

export interface ApiSchemaOptions {
  graphqlValidation?: boolean;
}

export function apiSchema(
  sdl: string,
  options: ApiSchemaOptions = {}
): ExecutionResult<String> {
  let schema: String;

  const validate = options.graphqlValidation ?? true;
  if (validate) {
    try {
      // First go through regular schema parsing
      buildGraphqlSchema(sdl);
    } catch (e) {
      return {
        errors: [e],
      };
    }
  }

  try {
    // Now try to get the API schema
    let [composedSchema] = buildSupergraphSchema(sdl);

    let apiSchema = composedSchema.toAPISchema();
    schema = printSchema(apiSchema.toGraphQLJSSchema());
  } catch (e) {
    e.supergraph = true;
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
