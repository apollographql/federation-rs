import {
  buildSchema as buildGraphqlSchema,
  ExecutionResult,
  GraphQLError,
  printSchema,
} from "graphql";

import { Supergraph } from "@apollo/federation-internals";
import { ROUTER_SUPPORTED_SUPERGRAPH_FEATURES } from "./supported_features";

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
    let supergraph = Supergraph.build(sdl, {
      supportedFeatures: ROUTER_SUPPORTED_SUPERGRAPH_FEATURES,
    });

    let apiSchema = supergraph.apiSchema();
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
