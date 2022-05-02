import { QueryPlan } from "@apollo/query-planner";
import { BridgeQueryPlanner, ExecutionResultWithUsageReporting } from "./plan";
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
  ExecutionResultWithUsageReporting<QueryPlan> | FatalError;

type FatalError = {
  errors: Error[];
};

const send = async (payload: WorkerResultWithId): Promise<void> => {
  logger.debug(`plan_worker: sending payload ${JSON.stringify(payload)}`);
  await Deno.core.opAsync("send", payload);
};
const receive = async (): Promise<PlannerEventWithId> =>
  await Deno.core.opAsync("receive");

let planner: BridgeQueryPlanner;

const updateQueryPlanner = (schema: string): WorkerResult => {
  try {
    planner = new bridge.BridgeQueryPlanner(schema);
    // This will be interpreted as a correct Update
    return { data: { kind: "QueryPlan", node: null } };
  } catch (err) {
    // The error that has been thrown needs to be sent back
    // to the rust runtime. In order to do so, it will be serialized.
    // The code below will allow us to build an object that is JSON serializable,
    // yet contains all of the information we need

    const errorArray = Array.isArray(err) ? err : [err];
    const errors = errorArray.map((err) => {
      // We can't import GraphqlError,
      // which would have been less hacky
      if (err?.extensions !== null && err?.extensions !== undefined) {
        return err;
      }

      const { name, message, stack } = err;
      return {
        name,
        message,
        stack,
      };
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
            const updateResult = updateQueryPlanner(event.schema);
            await send({ id, payload: updateResult });
            break;
          case PlannerEventKind.Plan:
            const planResult = planner.plan(event.query, event.operationName);
            await send({ id, payload: planResult });
            break;
          case PlannerEventKind.Exit:
            return;
          default:
            print(`unknown message received: ${JSON.stringify(event)}\n`);
            break;
        }
      } catch (e) {
        print(`an error happened in the worker runtime ${e}\n`);
        await send({ id, payload: { errors: [e] } });
      }
    } catch (e) {
      logger.warn(`plan_worker: an unknown error occured ${e}\n`);
      await send({ payload: { errors: [e] } });
    }
  }
}

run();
