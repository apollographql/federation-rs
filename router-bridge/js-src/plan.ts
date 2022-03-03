import { ExecutionResult, parse, validate } from "graphql";
import { QueryPlanner, QueryPlan } from "@apollo/query-planner";

import {
  buildSchema,
  operationFromDocument,
} from "@apollo/federation-internals";

export function plan(
  schemaString: string,
  operationString: string,
  operationName?: string
): ExecutionResult<QueryPlan> {
  try {
    const composedSchema = buildSchema(schemaString);
    const apiSchema = composedSchema.toAPISchema();
    const operationDocument = parse(operationString);
    const graphqlJsSchema = apiSchema.toGraphQLJSSchema();

    // Federation does some validation, but not all.  We need to do
    // all default validations that are provided by GraphQL.
    const validationErrors = validate(graphqlJsSchema, operationDocument);
    if (validationErrors.length > 0) {
      return { errors: validationErrors };
    }

    const operation = operationFromDocument(
      composedSchema,
      operationDocument,
      operationName
    );

    const planner = new QueryPlanner(composedSchema);
    return { data: planner.buildQueryPlan(operation) };
  } catch (e) {
    return { errors: [e] };
  }
}
