import {
  prettyFormatQueryPlan,
  QueryPlan,
  QueryPlanner,
  QueryPlannerConfig,
} from "@apollo/query-planner";
import {
  DocumentNode,
  ExecutionResult,
  GraphQLError,
  GraphQLSchema,
  parse,
  validate,
  printSchema,
  graphqlSync,
} from "graphql";

import {
  buildSupergraphSchema,
  Operation,
  operationFromDocument,
  Schema,
} from "@apollo/federation-internals";
import {
  calculateReferencedFieldsByType,
  usageReportingSignature,
} from "@apollo/utils.usagereporting";
import { ReferencedFieldsForType } from "apollo-reporting-protobuf";

const PARSE_FAILURE: string = "## GraphQLParseFailure\n";
const PARSE_FAILURE_EXT_CODE: string = "GRAPHQL_PARSE_FAILED";
const VALIDATION_FAILURE: string = "## GraphQLValidationFailure\n";
const VALIDATION_FAILURE_EXT_CODE: string = "GRAPHQL_VALIDATION_FAILED";
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

export interface QueryPlanResult {
  formattedQueryPlan: string;
  queryPlan: QueryPlan;
}

export class BridgeQueryPlanner {
  private readonly composedSchema: Schema;
  private readonly apiSchema: GraphQLSchema;
  private readonly planner: QueryPlanner;

  constructor(
    public readonly schemaString: string,
    public readonly options: QueryPlannerConfig
  ) {
    const [schema] = buildSupergraphSchema(schemaString);
    this.composedSchema = schema;
    const apiSchema = this.composedSchema.toAPISchema();
    this.apiSchema = apiSchema.toGraphQLJSSchema({
      includeDefer: options.incrementalDelivery?.enableDefer,
    });
    this.planner = new QueryPlanner(this.composedSchema, options);
  }

  plan(
    operationString: string,
    providedOperationName?: string
  ): ExecutionResultWithUsageReporting<QueryPlanResult> {
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
        errors: [
          {
            ...parseError,
            extensions: {
              code: PARSE_FAILURE_EXT_CODE,
            },
          },
        ],
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
        errors: validationErrors.map((error): GraphQLError => {
          if (
            error.extensions == null ||
            Object.keys(error.extensions).length === 0
          ) {
            return new GraphQLError(error.message, {
              extensions: {
                code: VALIDATION_FAILURE_EXT_CODE,
              },
              path: error.path,
              nodes: error.nodes,
              originalError: error.originalError,
              positions: error.positions,
              source: error.source,
            });
          }

          return error;
        }),
      };
    }

    let operation: Operation;
    try {
      operation = operationFromDocument(this.composedSchema, document, {
        operationName: providedOperationName,
      });
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
        errors: [
          {
            ...e,
            extensions: {
              code: VALIDATION_FAILURE_EXT_CODE,
            },
          },
        ],
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
    const queryPlan = this.planner.buildQueryPlan(operation);
    let formattedQueryPlan: string | null;
    try {
      formattedQueryPlan = prettyFormatQueryPlan(queryPlan);
    } catch (err) {
      // We have decided that since we HAVE a query plan (above), there is
      // absolutely no reason to interrupt the ability to proceed just because
      // we wanted a pretty-printed version of the query planner here.  Therefore
      // we will just proceed without the pretty printed bits.
      logger.warn(
        `Couldn't generate pretty query plan for ${
          operationName ? "operation " + operationName : "anonymous operation"
        }: ${err}`
      );
      formattedQueryPlan = null;
    }

    return {
      usageReporting: {
        statsReportKey,
        referencedFieldsByType: operationDerivedData.referencedFieldsByType,
      },
      data: {
        queryPlan,
        formattedQueryPlan,
      },
    };
  }

  getApiSchema(): string {
    return printSchema(this.apiSchema);
  }

  introspect(query: string): ExecutionResult {
    const { data, errors } = graphqlSync({
      schema: this.apiSchema,
      source: query,
    });

    if (errors) {
      return { data, errors: [...errors] };
    } else {
      return { data, errors: [] };
    }
  }
}

export function queryPlanner(
  schemaString: string,
  options: QueryPlannerConfig
): BridgeQueryPlanner {
  return new BridgeQueryPlanner(schemaString, options);
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
