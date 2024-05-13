import type { batchIntrospect } from ".";
import type { OperationResult, QueryPlannerConfigExt } from "./types";

/**
 * There are several global properties that we make available in our V8 runtime
 * and these are the types for those that we expect to use within this script.
 * They'll be stripped in the emitting of this file as JS, of course.
 */
declare let bridge: { batchIntrospect: typeof batchIntrospect };

declare let sdl: string;
declare let queries: string[];
declare let config: QueryPlannerConfigExt;

let opResult: OperationResult;
if (!sdl) {
  opResult = {
    Err: [{ message: "Error in JS-Rust-land: SDL is empty." }],
  };
} else {
  try {
    opResult = { Ok: bridge.batchIntrospect(sdl, queries, config) };
  } catch (err) {
    opResult = { Err: err };
  }
}
// The JsRuntime::execute_script Rust function will return this top-level value,
// because it is the final completion value of the current script.
opResult;
