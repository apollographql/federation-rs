import {
  DocumentNode,
  ExecutionResult,
  GraphQLSchema,
  parse,
  validate,
} from "graphql";
import { QueryPlanner, QueryPlan } from "@apollo/query-planner";

import {
  buildSupergraphSchema,
  operationFromDocument,
  Operation,
  Schema,
} from "@apollo/federation-internals";
import { ReferencedFieldsForType } from "apollo-reporting-protobuf";
import { usageReportingSignature } from "@apollo/utils.usagereportingsignature";
import { calculateReferencedFieldsByType } from "@apollo/utils.calculatereferencedfieldsbytype";

const PARSE_FAILURE: string = "## GraphQLParseFailure\n";
const VALIDATION_FAILURE: string = "## GraphQLValidationFailure\n";
const UNKNOWN_OPERATION: string = "## GraphQLUnknownOperationName\n";

export type ReferencedFieldsByType = Record<string, ReferencedFieldsForType>;

export type UsageReporting = {
  statsReportKey: string;
  referencedFieldsByType: ReferencedFieldsByType;
};
export interface ExecutionResultWithUsageReporting<T>
  extends ExecutionResult<T> {
  usageReporting: UsageReporting;
}

export class BridgeQueryPlanner {
  private readonly composedSchema: Schema;
  private readonly apiSchema: GraphQLSchema;
  private readonly planner: QueryPlanner;

  constructor(public readonly schemaString: string) {
    const [schema] = buildSupergraphSchema(schemaString);
    this.composedSchema = schema;
    const apiSchema = this.composedSchema.toAPISchema();
    this.apiSchema = apiSchema.toGraphQLJSSchema();
    this.planner = new QueryPlanner(this.composedSchema);
  }

  plan(
    operationString: string,
    providedOperationName?: string
  ): ExecutionResultWithUsageReporting<QueryPlan> {
    let document: DocumentNode;

    try {
      document = parse(operationString);
    } catch (parseError) {
      // parse throws GraphQLError
      return {
        usageReporting: {
          statsReportKey: PARSE_FAILURE,
          referencedFieldsByType: {},
        },
        errors: [parseError],
      };
    }

    // Federation does some validation, but not all.  We need to do
    // all default validations that are provided by GraphQL.
    const validationErrors = validate(this.apiSchema, document);
    if (validationErrors.length > 0) {
      return {
        usageReporting: {
          statsReportKey: VALIDATION_FAILURE,
          referencedFieldsByType: {},
        },
        errors: validationErrors,
      };
    }

    let operation: Operation;
    try {
      operation = operationFromDocument(
        this.composedSchema,
        document,
        providedOperationName
      );
    } catch (e) {
      // operationFromDocument throws GraphQLError

      let statsReportKey = VALIDATION_FAILURE;

      if (
        e.message.startsWith("Unknown operation named") ||
        e.message.startsWith("Must provide operation name")
      ) {
        statsReportKey = UNKNOWN_OPERATION;
      }

      return {
        usageReporting: {
          statsReportKey,
          referencedFieldsByType: {},
        },
        errors: [e],
      };
    }

    // Adapted from here
    // https://github.com/apollographql/apollo-server/blob/444c403011209023b5d3b5162b8fb81991046b23/packages/apollo-server-core/src/requestPipeline.ts#L303
    const operationName = operation?.name;

    // I double checked, this function doesn't throw
    const operationDerivedData = getOperationDerivedData({
      schema: this.apiSchema,
      document,
      operationName,
    });

    const statsReportKey = `# ${operationName || "-"}\n${
      operationDerivedData.signature
    }`;

    return {
      usageReporting: {
        statsReportKey,
        referencedFieldsByType: operationDerivedData.referencedFieldsByType,
      },
      data: this.planner.buildQueryPlan(operation),
    };
  }
}

export function queryPlanner(schemaString: string): BridgeQueryPlanner {
  return new BridgeQueryPlanner(schemaString);
}

// ---------------------

// Interface definition copied from here
// https://github.com/apollographql/apollo-server/blob/d75c6cf3360a46ebcd944b2113438be8f549ae6f/packages/apollo-server-core/src/plugin/usageReporting/operationDerivedDataCache.ts#L5
export interface OperationDerivedData {
  signature: string;
  referencedFieldsByType: ReferencedFieldsByType;
}

function getOperationDerivedData({
  schema,
  document,
  operationName,
}: {
  schema: GraphQLSchema;
  document: DocumentNode;
  operationName: string;
}): OperationDerivedData {
  const generatedSignature = usageReportingSignature(
    document,
    operationName || ""
  );

  const generatedOperationDerivedData: OperationDerivedData = {
    signature: generatedSignature,
    referencedFieldsByType: calculateReferencedFieldsByType({
      document,
      schema,
      resolvedOperationName: operationName ?? null,
    }),
  };
  return generatedOperationDerivedData;
}
