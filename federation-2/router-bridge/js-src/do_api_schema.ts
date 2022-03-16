import type { apiSchema } from ".";
import type { OperationResult } from "./types";

/**
 * There are several global properties that we make available in our V8 runtime
 * and these are the types for those that we expect to use within this script.
 * They'll be stripped in the emitting of this file as JS, of course.
 */
declare let bridge: { apiSchema: typeof apiSchema };

declare let done: (operationResult: OperationResult) => void;
declare let sdl: string;

const result = bridge.apiSchema(sdl);

if (result.errors?.length > 0) {
  done({ Err: result.errors });
} else {
  done({ Ok: result.data });
}
