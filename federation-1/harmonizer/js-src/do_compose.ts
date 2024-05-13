import { composition } from ".";
import type { CompositionResult } from "./types";

/**
 * There are several global properties that we make available in our V8 runtime
 * and these are the types for those that we expect to use within this script.
 * They'll be stripped in the emitting of this file as JS, of course.
 */
declare let composition_bridge: { composition: typeof composition };
declare let serviceList: { sdl: string; name: string; url: string }[];

let result: CompositionResult;
try {
  result = composition_bridge.composition(serviceList);
} catch (err) {
  result = { Err: [err] };
}
// The JsRuntime::execute_script Rust function will return this top-level value,
// because it is the final completion value of the current script.
result;
