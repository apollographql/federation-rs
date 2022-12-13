import { composition } from ".";
import type { CompositionResult } from "./types";

/**
 * There are several global properties that we make available in our V8 runtime
 * and these are the types for those that we expect to use within this script.
 * They'll be stripped in the emitting of this file as JS, of course.
 */
declare let composition_bridge: { composition: typeof composition };

declare let done: (compositionResult: CompositionResult) => void;
declare let serviceList: { sdl: string; name: string; url?: string }[];

try {
  // /**
  //  * @type {{ errors: Error[], supergraphSdl?: undefined, hints: undefined } | { errors?: undefined, supergraphSdl: string, hints: string }}
  //  */
  const composed = composition_bridge.composition(serviceList);
  /**
   * @type {BuildHint[]}
   */
  let hints: { message: string }[] = [];
  if (composed.hints) {
    composed.hints.map((composed_hint) => {
      hints.push({ message: composed_hint.toString() });
    });
  }
  done(
    composed.errors
      ? { Err: composed.errors }
      : {
          Ok: {
            supergraphSdl: composed.supergraphSdl,
            hints,
          },
        }
  );
} catch (err) {
  done({ Err: [{ message: err.toString() }] });
}
