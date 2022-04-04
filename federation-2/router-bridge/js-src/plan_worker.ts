import { QueryPlan } from "@apollo/query-planner";
import { ExecutionResult } from "graphql";
import { BridgeQueryPlanner } from "./plan";
declare let bridge: { BridgeQueryPlanner: typeof BridgeQueryPlanner };
// Todo: there sure is a better  way to deal with this huh.
declare let Deno: { core: { opAsync: any; opSync: any } };

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
type WorkerResult =
  // Plan result
  ExecutionResult<QueryPlan> | FatalError;

type FatalError = {
  errors: Error[];
};

const send = async (payload: WorkerResult): Promise<void> =>
  await Deno.core.opAsync("send", payload);
const receive = async (): Promise<PlannerEvent> =>
  await Deno.core.opAsync("receive");

let planner: BridgeQueryPlanner;

const updateQueryPlanner = (schema: string): WorkerResult => {
  try {
    planner = new bridge.BridgeQueryPlanner(schema);
    // This will be interpreted as a correct Update
    return { data: { kind: "QueryPlan", node: null } };
  } catch (e) {
    const errors = Array.isArray(e) ? e : [e];
    return { errors };
  }
};

async function run() {
  while (true) {
    try {
      const event = await receive();
      switch (event?.kind) {
        case PlannerEventKind.UpdateSchema:
          const updateResult = updateQueryPlanner(event.schema);
          await send(updateResult);
          break;
        case PlannerEventKind.Plan:
          const result = planner.plan(event.query, event.operationName);
          await send(result);
          break;
        case PlannerEventKind.Exit:
          return;
        default:
          print(`unknown message received: ${JSON.stringify(event)}\n`);
          break;
      }
    } catch (e) {
      print(`received error ${e}\n`);
      await send({ errors: [e] });
    }
  }
}

run();
