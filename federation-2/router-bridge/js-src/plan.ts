import {
  DocumentNode,
  ExecutionResult,
  GraphQLSchema,
  isInterfaceType,
  parse,
  TypeInfo,
  validate,
  visitWithTypeInfo,
} from "graphql";
import { QueryPlanner, QueryPlan } from "@apollo/query-planner";

import {
  buildSupergraphSchema,
  operationFromDocument,
  Operation,
  Schema,
} from "@apollo/federation-internals";
import { ReferencedFieldsForType } from "apollo-reporting-protobuf";

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
  usageReporting?: UsageReporting;
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
      const statsReportKey = e.message.startsWith("Unknown operation named")
        ? UNKNOWN_OPERATION
        : VALIDATION_FAILURE;
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
  const generatedSignature = defaultUsageReportingSignature(
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

// ---------------------

// Adapted from here
// https://github.com/apollographql/apollo-server/blob/06dd1171562564f0ca180e74a6fa5530e0389076/packages/apollo-server-core/src/plugin/usageReporting/referencedFields.ts

export function calculateReferencedFieldsByType({
  document,
  schema,
  resolvedOperationName,
}: {
  document: DocumentNode;
  resolvedOperationName: string | null;
  schema: GraphQLSchema;
}): ReferencedFieldsByType {
  // If the document contains multiple operations, we only care about fields
  // referenced in the operation we're using and in fragments that are
  // (transitively) spread by that operation. (This is because Studio's field
  // usage accounting is all by operation, not by document.) This does mean that
  // a field can be textually present in a GraphQL document (and need to exist
  // for validation) without being represented in the reported referenced fields
  // structure, but we'd need to change the data model of Studio to be based on
  // documents rather than fields if we wanted to improve that.
  const documentSeparatedByOperation = separateOperations(document);
  const filteredDocument =
    documentSeparatedByOperation[resolvedOperationName ?? ""];
  if (!filteredDocument) {
    // This shouldn't happen because we only should call this function on
    // properly executable documents.
    throw Error(
      `shouldn't happen: operation '${resolvedOperationName ?? ""}' not found`
    );
  }
  const typeInfo = new TypeInfo(schema);
  const interfaces = new Set<string>();
  const referencedFieldSetByType: Record<string, Set<string>> = Object.create(
    null
  );
  visit(
    filteredDocument,
    visitWithTypeInfo(typeInfo, {
      Field(field) {
        const fieldName = field.name.value;
        const parentType = typeInfo.getParentType();
        if (!parentType) {
          throw Error(
            `shouldn't happen: missing parent type for field ${fieldName}`
          );
        }
        const parentTypeName = parentType.name;
        if (!referencedFieldSetByType[parentTypeName]) {
          referencedFieldSetByType[parentTypeName] = new Set<string>();
          if (isInterfaceType(parentType)) {
            interfaces.add(parentTypeName);
          }
        }
        referencedFieldSetByType[parentTypeName].add(fieldName);
      },
    })
  );

  // Convert from initial representation (which uses Sets to avoid quadratic
  // behavior) to the protobufjs objects. (We could also use js_use_toArray here
  // but that seems a little overkill.)
  const referencedFieldsByType = Object.create(null);
  for (const [typeName, fieldNames] of Object.entries(
    referencedFieldSetByType
  )) {
    referencedFieldsByType[typeName] = new ReferencedFieldsForType({
      fieldNames: [...fieldNames],
      isInterface: interfaces.has(typeName),
    });
  }
  return referencedFieldsByType;
}

// ---------------------

// Copied from here
// https://github.com/apollographql/apollo-server/blob/bf2f70d74b27b0862ad5d282a153f935e0bde5cc/packages/apollo-server-core/src/plugin/usageReporting/defaultUsageReportingSignature.ts

// In Apollo Studio, we want to group requests making the same query together,
// and treat different queries distinctly. But what does it mean for two queries
// to be "the same"?  And what if you don't want to send the full text of the
// query to Apollo's servers, either because it contains sensitive data or
// because it contains extraneous operations or fragments?
//
// To solve these problems, ApolloServerPluginUsageReporting has the concept of
// "signatures". We don't (by default) send the full query string of queries to
// Apollo's servers. Instead, each trace has its query string's "signature".
//
// You can technically specify any function mapping a GraphQL query AST
// (DocumentNode) to string as your signature algorithm by providing it as the
// 'calculateSignature' option to ApolloServerPluginUsageReporting. (This option
// is not recommended, because Apollo's servers make some assumptions about the
// semantics of your operation based on the signature.) This file defines the
// default function used for this purpose: defaultUsageReportingSignature
// (formerly known as defaultEngineReportingSignature).
//
// This module utilizes several AST transformations from the adjacent
// 'transforms' file. (You could use them to build your own `calculateSignature`
// callback, but as mentioned above, you shouldn't really define that callback,
// so they are not exported from the package.)

// - dropUnusedDefinitions, which removes operations and fragments that aren't
//   going to be used in execution
// - hideLiterals, which replaces all numeric and string literals as well as
//   list and object input values with "empty" values
// - removeAliases, which removes field aliasing from the query
// - sortAST, which sorts the children of most multi-child nodes consistently
// - printWithReducedWhitespace, a variant on graphql-js's 'print' which gets
//   rid of unneeded whitespace
//
// defaultUsageReportingSignature consists of applying all of these building
// blocks.
//
// Historical note: the default signature algorithm of the Go engineproxy
// performed all of the above operations, and Apollo's servers then re-ran a
// mostly identical signature implementation on received traces. This was
// primarily to deal with edge cases where some users used literal interpolation
// instead of GraphQL variables, included randomized alias names, etc. In
// addition, the servers relied on the fact that dropUnusedDefinitions had been
// called in order (and that the signature could be parsed as GraphQL) to
// extract the name of the operation for display. This caused confusion, as the
// query document shown in the Studio UI wasn't the same as the one actually
// sent. ApolloServerPluginUsageReporting (previously apollo-engine-reporting)
// uses a reporting API which requires it to explicitly include the operation
// name with each signature; this means that the server no longer needs to parse
// the signature or run its own signature algorithm on it, and the details of
// the signature algorithm are now up to the reporting agent. That said, not all
// Studio features will work properly if your signature function changes the
// signature in unexpected ways.
//
// This function used to live in the `apollo-graphql` package in the
// `apollo-tooling` repository.
//
// Note that this is not exactly the same algorithm as the operation registry
// signature function, which continues to live in `apollo-graphql`.
import {
  DirectiveNode,
  FieldNode,
  FloatValueNode,
  FragmentDefinitionNode,
  FragmentSpreadNode,
  InlineFragmentNode,
  IntValueNode,
  ListValueNode,
  ObjectValueNode,
  OperationDefinitionNode,
  print,
  SelectionSetNode,
  separateOperations,
  StringValueNode,
  visit,
} from "graphql";
import sortBy from "lodash.sortby";
import { Buffer } from "buffer";

export function defaultUsageReportingSignature(
  ast: DocumentNode,
  operationName: string
): string {
  return printWithReducedWhitespace(
    sortAST(
      removeAliases(hideLiterals(dropUnusedDefinitions(ast, operationName)))
    )
  );
}

// Like the graphql-js print function, but deleting whitespace wherever
// feasible. Specifically, all whitespace (outside of string literals) is
// reduced to at most one space, and even that space is removed anywhere except
// for between two alphanumerics.
//
// Note that recent versions of graphql-js contain a stripIgnoredCharacters
// function; it would be better to use that instead, though whenever we change
// the signature algorithm it does make every operation appear to change in
// Studio.
//
// In a GraphQL AST (which notably does not contain comments), the only place
// where meaningful whitespace (or double quotes) can exist is in StringNodes.
// So to print with reduced whitespace, we:
// - temporarily sanitize strings by replacing their contents with hex
// - use the default GraphQL printer
// - minimize the whitespace with a simple regexp replacement
// - convert strings back to their actual value We normalize all strings to
//   non-block strings for simplicity.
// (Unlike stripIgnoredCharacters, this does not remove commas.)
function printWithReducedWhitespace(ast: DocumentNode): string {
  const sanitizedAST = visit(ast, {
    StringValue(node: StringValueNode): StringValueNode {
      return {
        ...node,
        value: Buffer.from(node.value, "utf8").toString("hex"),
        block: false,
      };
    },
  });
  const withWhitespace = print(sanitizedAST);
  const minimizedButStillHex = withWhitespace
    .replace(/\s+/g, " ")
    .replace(/([^_a-zA-Z0-9]) /g, (_, c) => c)
    .replace(/ ([^_a-zA-Z0-9])/g, (_, c) => c);
  return minimizedButStillHex.replace(/"([a-f0-9]+)"/g, (_, hex) =>
    JSON.stringify(Buffer.from(hex, "hex").toString("utf8"))
  );
}

// sortAST sorts most multi-child nodes alphabetically. Using this as part of
// your signature calculation function may make it easier to tell the difference
// between queries that are similar to each other, and if for some reason your
// GraphQL client generates query strings with elements in nondeterministic
// order, it can make sure the queries are treated as identical.
function sortAST(ast: DocumentNode): DocumentNode {
  return visit(ast, {
    Document(node: DocumentNode) {
      return {
        ...node,
        // Use sortBy instead of sorted because 'definitions' is not optional.
        // The sort on "kind" places fragments before operations within the document
        definitions: sortBy(node.definitions, "kind", "name.value"),
      };
    },
    OperationDefinition(
      node: OperationDefinitionNode
    ): OperationDefinitionNode {
      return {
        ...node,
        variableDefinitions: sorted(
          node.variableDefinitions,
          "variable.name.value"
        ),
      };
    },
    SelectionSet(node: SelectionSetNode): SelectionSetNode {
      return {
        ...node,
        // Define an ordering for field names in a SelectionSet.  Field first,
        // then FragmentSpread, then InlineFragment.  By a lovely coincidence,
        // the order we want them to appear in is alphabetical by node.kind.
        // Use sortBy instead of sorted because 'selections' is not optional.
        selections: sortBy(node.selections, "kind", "name.value"),
      };
    },
    Field(node: FieldNode): FieldNode {
      return {
        ...node,
        arguments: sorted(node.arguments, "name.value"),
      };
    },
    FragmentSpread(node: FragmentSpreadNode): FragmentSpreadNode {
      return { ...node, directives: sorted(node.directives, "name.value") };
    },
    InlineFragment(node: InlineFragmentNode): InlineFragmentNode {
      return { ...node, directives: sorted(node.directives, "name.value") };
    },
    FragmentDefinition(node: FragmentDefinitionNode): FragmentDefinitionNode {
      return {
        ...node,
        directives: sorted(node.directives, "name.value"),
        variableDefinitions: sorted(
          node.variableDefinitions,
          "variable.name.value"
        ),
      };
    },
    Directive(node: DirectiveNode): DirectiveNode {
      return { ...node, arguments: sorted(node.arguments, "name.value") };
    },
  });
}

// Like lodash's sortBy, but sorted(undefined) === undefined rather than []. It
// is a stable non-in-place sort.
function sorted<T>(
  items: ReadonlyArray<T> | undefined,
  ...iteratees: string[]
): T[] | undefined {
  if (items) {
    return sortBy(items, ...iteratees);
  }
  return undefined;
}

// removeAliases gets rid of GraphQL aliases, a feature by which you can tell a
// server to return a field's data under a different name from the field
// name. Maybe this is useful if somebody somewhere inserts random aliases into
// their queries.
function removeAliases(ast: DocumentNode): DocumentNode {
  return visit(ast, {
    Field(node: FieldNode): FieldNode {
      return {
        ...node,
        alias: undefined,
      };
    },
  });
}

// Replace numeric, string, list, and object literals with "empty"
// values. Leaves enums alone (since there's no consistent "zero" enum). This
// can help combine similar queries if you substitute values directly into
// queries rather than use GraphQL variables, and can hide sensitive data in
// your query (say, a hardcoded API key) from Apollo's servers, but in general
// avoiding those situations is better than working around them.
function hideLiterals(ast: DocumentNode): DocumentNode {
  return visit(ast, {
    IntValue(node: IntValueNode): IntValueNode {
      return { ...node, value: "0" };
    },
    FloatValue(node: FloatValueNode): FloatValueNode {
      return { ...node, value: "0" };
    },
    StringValue(node: StringValueNode): StringValueNode {
      return { ...node, value: "", block: false };
    },
    ListValue(node: ListValueNode): ListValueNode {
      return { ...node, values: [] };
    },
    ObjectValue(node: ObjectValueNode): ObjectValueNode {
      return { ...node, fields: [] };
    },
  });
}

// A GraphQL query may contain multiple named operations, with the operation to
// use specified separately by the client. This transformation drops unused
// operations from the query, as well as any fragment definitions that are not
// referenced.  (In general we recommend that unused definitions are dropped on
// the client before sending to the server to save bandwidth and parsing time.)
//
// This has the unfortunate side effect that Studio usage reporting never finds
// out about fields that are referenced in operations and fragments in a request
// that are not executed, so (for example) schema checks don't understand that
// deleting those fields would make the client's request fail to validate.
function dropUnusedDefinitions(
  ast: DocumentNode,
  operationName: string
): DocumentNode {
  const separated = separateOperations(ast)[operationName];
  if (!separated) {
    // If the given operationName isn't found, just make this whole transform a
    // no-op instead of crashing.
    return ast;
  }
  return separated;
}
