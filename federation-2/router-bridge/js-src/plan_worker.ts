import { GraphQLErrorExt } from "@apollo/core-schema/dist/error";
import { QueryPlannerConfig } from "@apollo/query-planner";
import { ASTNode, Source, SourceLocation } from "graphql";
import {
  BridgeQueryPlanner,
  ExecutionResultWithUsageReporting,
  QueryPlanResult,
} from "./plan";
declare let bridge: { BridgeQueryPlanner: typeof BridgeQueryPlanner };
declare let Deno: { core: { opAsync: any; opSync: any } };
let logFunction: (message: string) => void;
declare let logger: {
  trace: typeof logFunction;
  debug: typeof logFunction;
  info: typeof logFunction;
  warn: typeof logFunction;
  error: typeof logFunction;
};

enum PlannerEventKind {
  UpdateSchema = "UpdateSchema",
  Plan = "Plan",
  Exit = "Exit",
}
interface UpdateSchemaEvent {
  kind: PlannerEventKind.UpdateSchema;
  schema: string;
  config: QueryPlannerConfig;
}
interface PlanEvent {
  kind: PlannerEventKind.Plan;
  query: string;
  operationName?: string;
}
interface Exit {
  kind: PlannerEventKind.Exit;
}
type PlannerEvent = UpdateSchemaEvent | PlanEvent | Exit;
type PlannerEventWithId = {
  id: string;
  payload: PlannerEvent;
};

type WorkerResultWithId = {
  id?: string;
  payload: WorkerResult;
};
type WorkerResult =
  // Plan result
  ExecutionResultWithUsageReporting<QueryPlanResult> | FatalError;

type FatalError = {
  errors: (JsError | WorkerGraphQLError)[];
};

type JsError = {
  name: string;
  message: string;
  stack?: string;
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
};
const isGraphQLErrorExt = (error: any): error is GraphQLErrorExt<string> =>
  error.name === "GraphQLError" || error.name === "CheckFailed";

const intoSerializableError = (error: Error): JsError => {
  const { name, message, stack } = error;
  return {
    name,
    message,
    stack,
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
  error: GraphQLErrorExt<string>
): WorkerGraphQLError => {
  const { message, locations, path, extensions } = error.toJSON();
  const { nodes, source, positions, originalError, name } = error;
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
  };
};

const send = async (payload: WorkerResultWithId): Promise<void> => {
  logger.trace(`plan_worker: sending payload ${JSON.stringify(payload)}`);
  await Deno.core.opAsync("send", payload);
};
const receive = async (): Promise<PlannerEventWithId> =>
  await Deno.core.opAsync("receive");

let planner: BridgeQueryPlanner;

const updateQueryPlanner = (
  schema: string,
  options: QueryPlannerConfig
): WorkerResult => {
  try {
    planner = new bridge.BridgeQueryPlanner(schema, options);
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
    try {
      const { id, payload: event } = await receive();
      try {
        switch (event?.kind) {
          case PlannerEventKind.UpdateSchema:
            const updateResult = updateQueryPlanner(event.schema, event.config);
            await send({ id, payload: updateResult });
            break;
          case PlannerEventKind.Plan:
            const planResult = planner.plan(event.query, event.operationName);
            await send({ id, payload: planResult });
            break;
          case PlannerEventKind.Exit:
            return;
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
              stacktrace: e.toString().split(/\n/),
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
            stacktrace: e.toString().split(/\n/),
          },
        },
      };

      await send({
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
