import { ExecutionResult, GraphQLSchema, parse, validate } from "graphql";
import { QueryPlanner, QueryPlan } from "@apollo/query-planner";

import {
  buildSchema,
  operationFromDocument,
  Schema,
} from "@apollo/federation-internals";

export class BridgeQueryPlanner {
  private readonly composedSchema: Schema;
  private readonly apiSchema: GraphQLSchema;
  private readonly planner: QueryPlanner;

  constructor(public readonly schemaString: string) {
    this.composedSchema = buildSchema(schemaString);
    const apiSchema = this.composedSchema.toAPISchema();
    this.apiSchema = apiSchema.toGraphQLJSSchema();
    this.planner = new QueryPlanner(this.composedSchema);
  }

  plan(
    operationString: string,
    operationName?: string
  ): ExecutionResult<QueryPlan> {
    try {
      const operationDocument = parse(operationString);

      // Federation does some validation, but not all.  We need to do
      // all default validations that are provided by GraphQL.
      const validationErrors = validate(this.apiSchema, operationDocument);
      if (validationErrors.length > 0) {
        throw validationErrors;
      }

      const operation = operationFromDocument(
        this.composedSchema,
        operationDocument,
        operationName
      );

      return { data: this.planner.buildQueryPlan(operation) };
    } catch (e) {
      const errors = Array.isArray(e) ? e : [e];
      return { errors };
    }
  }
}

export function queryPlanner(schemaString: string): BridgeQueryPlanner {
  return new BridgeQueryPlanner(schemaString);
}

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
