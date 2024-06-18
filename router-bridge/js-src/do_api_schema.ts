import type { apiSchema } from ".";
import type { OperationResult } from "./types";

/**
 * There are several global properties that we make available in our V8 runtime
 * and these are the types for those that we expect to use within this script.
 * They'll be stripped in the emitting of this file as JS, of course.
 */
declare let bridge: { apiSchema: typeof apiSchema };

declare let sdl: string;
declare let graphqlValidation: boolean | undefined;

const result = bridge.apiSchema(sdl, { graphqlValidation });

let opResult: OperationResult;
if (result.errors?.length > 0) {
  opResult = { Err: result.errors };
} else {
  opResult = { Ok: result.data };
}
// The JsRuntime::execute_script Rust function will return this top-level value,
// because it is the final completion value of the current script.
opResult;
