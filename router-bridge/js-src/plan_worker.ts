import { GraphQLErrorExt } from "@apollo/core-schema/dist/error";
import { ASTNode, Source, SourceLocation, ExecutionResult } from "graphql";
import {
  BridgeQueryPlanner,
  ExecutionResultWithUsageReporting,
  PlanOptions,
  QueryPlanResult,
} from "./plan";
import { QueryPlannerConfigExt } from "./types";
declare let bridge: { BridgeQueryPlanner: typeof BridgeQueryPlanner };
declare namespace Deno {
  namespace core {
    function opAsync(opName: string, ...args: any[]): Promise<any>;
    const ops: Record<string, (...args: unknown[]) => any>;
  }
}

function memoryUsage(): MemoryUsage {
  return Deno.core.ops.op_runtime_memory_usage();
}

let logFunction: (message: string) => void;
declare let logger: {
  trace: typeof logFunction;
  debug: typeof logFunction;
  info: typeof logFunction;
  warn: typeof logFunction;
  error: typeof logFunction;
};

export interface MemoryUsage {
  /* The number of bytes of the current Deno's process resident set size,
   * which is the amount of memory occupied in main memory (RAM). */
  //rss: number;
  /** The total size of the heap for V8, in bytes. */
  heapTotal: number;
  /** The amount of the heap used for V8, in bytes. */
  heapUsed: number;
  /** Memory, in bytes, associated with JavaScript objects outside of the
   * JavaScript isolate. */
  external: number;
}

enum PlannerEventKind {
  UpdateSchema = "UpdateSchema",
  Plan = "Plan",
  Exit = "Exit",
  ApiSchema = "ApiSchema",
  Introspect = "Introspect",
  Signature = "Signature",
  Subgraphs = "Subgraphs",
  GetHeapStatistics = "GetHeapStatistics",
}

interface UpdateSchemaEvent {
  kind: PlannerEventKind.UpdateSchema;
  schema: string;
  config: QueryPlannerConfigExt;
  schemaId: number;
}

interface PlanEvent {
  kind: PlannerEventKind.Plan;
  query: string;
  operationName?: string;
  schemaId: number;
  options?: PlanOptions;
}
interface ApiSchemaEvent {
  kind: PlannerEventKind.ApiSchema;
  schemaId: number;
}

interface IntrospectEvent {
  kind: PlannerEventKind.Introspect;
  query: string;
  schemaId: number;
}

interface SignatureEvent {
  kind: PlannerEventKind.Signature;
  query: string;
  operationName?: string;
  schemaId: number;
}

interface SubgraphsEvent {
  kind: PlannerEventKind.Subgraphs;
  schemaId: number;
}

interface Exit {
  kind: PlannerEventKind.Exit;
  schemaId: number;
}

interface GetHeapStatisticsEvent {
  kind: PlannerEventKind.GetHeapStatistics;
}

type PlannerEvent =
  | UpdateSchemaEvent
  | PlanEvent
  | ApiSchemaEvent
  | IntrospectEvent
  | SignatureEvent
  | SubgraphsEvent
  | GetHeapStatisticsEvent
  | Exit;
type PlannerEventWithId = {
  id: string;
  payload: PlannerEvent;
};

type WorkerResultWithId = {
  id?: string;
  payload: WorkerResult;
};

type WorkerResult =
  | PlanResult
  | ApiSchemaResult
  | ExecutionResult
  | Map<string, string>
  | String
  | MemoryUsageResult;
// Plan result
type PlanResult =
  | ExecutionResultWithUsageReporting<QueryPlanResult>
  | FatalError;
type ApiSchemaResult = {
  schema: string;
};
type MemoryUsageResult = {
  //rss: number;
  heapTotal: number;
  heapUsed: number;
  external: number;
};

type FatalError = {
  errors: (JsError | WorkerGraphQLError)[];
};

type JsError = {
  name: string;
  message: string;
  stack?: string;
  validationError?: boolean;
};

type CauseError = {
  message: string;
  locations?: ReadonlyArray<SourceLocation>;
  extensions: {
    [key: string]: unknown;
  };
};

type WorkerGraphQLError = {
  name: string;
  message: string;
  locations?: ReadonlyArray<SourceLocation>;
  path?: ReadonlyArray<string | number>;
  extensions: {
    [key: string]: unknown;
  };
  nodes?: ReadonlyArray<ASTNode> | ASTNode;
  source?: Source;
  positions?: ReadonlyArray<number>;
  originalError?: Error;
  causes?: CauseError[];
  validationError?: boolean;
};
const isGraphQLErrorExt = (error: any): error is GraphQLErrorExt<string> =>
  error.name === "GraphQLError" || error.name === "CheckFailed";

const intoSerializableError = (error: Error): JsError => {
  const {
    name,
    message,
    stack,
    validationError = false,
  } = error as Error & { validationError?: boolean };
  return {
    name,
    message,
    stack,
    validationError,
  };
};

const intoCauseError = (error: any): CauseError => {
  const { locations, message, extensions } = error;
  return {
    locations,
    message,
    extensions,
  };
};

const intoSerializableGraphQLErrorExt = (
  error: GraphQLErrorExt<string> & { validationError?: boolean }
): WorkerGraphQLError => {
  const { message, locations, path, extensions } = error.toJSON();
  const {
    nodes,
    source,
    positions,
    originalError,
    name,
    validationError = false,
  } = error;
  const causes = (error as any).causes;
  return {
    name,
    message,
    locations,
    path,
    extensions,
    nodes,
    source,
    positions,
    originalError:
      originalError === undefined
        ? originalError
        : intoSerializableError(originalError),
    causes: causes === undefined ? causes : causes.map(intoCauseError),
    validationError,
  };
};

const send = async (payload: WorkerResultWithId): Promise<void> => {
  logger.trace(`plan_worker: sending payload ${JSON.stringify(payload)}`);
  await Deno.core.ops.send(payload);
};
const receive = async (): Promise<PlannerEventWithId> =>
  await Deno.core.ops.receive();

let planners = new Map<number, BridgeQueryPlanner>();

const updateQueryPlanner = (
  schema: string,
  options: QueryPlannerConfigExt,
  schemaId: number
): WorkerResult => {
  try {
    planners.set(schemaId, new bridge.BridgeQueryPlanner(schema, options));
    // This will be interpreted as a correct Update
    return {
      data: {
        queryPlan: { kind: "QueryPlan", node: null },
        formattedQueryPlan: "QueryPlan {}",
      },
      usageReporting: {
        statsReportKey: "",
        referencedFieldsByType: {},
      },
    };
  } catch (err) {
    // The error that has been thrown needs to be sent back
    // to the rust runtime. In order to do so, it will be serialized.
    // The code below will allow us to build an object that is JSON serializable,
    // yet contains all of the information we need
    const errorArray = Array.isArray(err) ? err : [err];
    const errors = errorArray.map((err) => {
      if (isGraphQLErrorExt(err)) {
        return intoSerializableGraphQLErrorExt(err);
      } else {
        return intoSerializableError(err);
      }
    });

    return { errors };
  }
};

async function run() {
  while (true) {
    // If an Exception is raised,
    // there won't be any messageId to send a payload back to the router,
    // unless we keep it in this scope.
    let messageId = "";
    try {
      const { id, payload: event } = await receive();
      messageId = id;

      try {
        switch (event?.kind) {
          case PlannerEventKind.UpdateSchema:
            const updateResult = updateQueryPlanner(
              event.schema,
              event.config,
              event.schemaId
            );
            await send({ id, payload: updateResult });
            break;
          case PlannerEventKind.Plan:
            const planResult = planners
              .get(event.schemaId)
              .plan(event.query, event.operationName, event.options);
            await send({ id, payload: planResult });
            break;
          case PlannerEventKind.ApiSchema:
            const apiSchemaResult = planners.get(event.schemaId).getApiSchema();
            const payload: ApiSchemaResult = { schema: apiSchemaResult };
            await send({ id, payload });
            break;
          case PlannerEventKind.Introspect:
            const introspectResult = planners
              .get(event.schemaId)
              .introspect(event.query);
            await send({ id, payload: introspectResult });
            break;
          case PlannerEventKind.Signature:
            const signature = planners
              .get(event.schemaId)
              .operationSignature(event.query, event.operationName);
            await send({ id, payload: signature });
            break;
          case PlannerEventKind.Subgraphs:
            const subgraphs = planners.get(event.schemaId).subgraphs();

            await send({ id, payload: subgraphs });
            break;
          case PlannerEventKind.GetHeapStatistics:
            logger.info(`received event: ${JSON.stringify(event)}`);

            //const mem = Deno.memoryUsage();
            const mem = memoryUsage();

            logger.info(`got memoryUsage`);
            const result: MemoryUsageResult = {
              //rss: mem.rss,
              heapTotal: mem.heapTotal,
              heapUsed: mem.heapUsed,
              external: mem.external,
            };
            /*const result: MemoryUsageResult = {
              rss: 1,
              heapTotal: 2,
              heapUsed: 3,
              external: 4,
            };*/
            logger.info(`memoryUsage: ${JSON.stringify(result)}`);

            await send({ id, payload: result });
            break;
          case PlannerEventKind.Exit:
            planners.delete(event.schemaId);
            if (planners.size == 0) {
              return;
            } else {
              break;
            }
          default:
            logger.warn(`unknown message received: ${JSON.stringify(event)}\n`);
            break;
        }
      } catch (e) {
        logger.warn(`an error happened in the worker runtime ${e}\n`);

        const unexpectedError = {
          name: e.name || "unknown",
          message: e.message || "",
          extensions: {
            code: "QUERY_PLANNING_FAILED",
            exception: {
              stacktrace: e.toString(),
            },
          },
        };

        await send({
          id,
          payload: {
            errors: [unexpectedError],
            usageReporting: {
              statsReportKey: "",
              referencedFieldsByType: {},
            },
          },
        });
      }
    } catch (e) {
      logger.warn(`plan_worker: an unknown error occurred ${e}\n`);

      const unexpectedError = {
        name: e.name || "unknown",
        message: e.message || "",
        extensions: {
          code: "QUERY_PLANNING_FAILED",
          exception: {
            stacktrace: e.toString(),
          },
        },
      };

      await send({
        id: messageId,
        payload: {
          errors: [unexpectedError],
          usageReporting: {
            statsReportKey: "",
            referencedFieldsByType: {},
          },
        },
      });
    }
  }
}

run();
