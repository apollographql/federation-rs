import { composition } from ".";
import type { CompositionResult } from "./types";

/**
 * There are several global properties that we make available in our V8 runtime
 * and these are the types for those that we expect to use within this script.
 * They'll be stripped in the emitting of this file as JS, of course.
 */
declare let composition_bridge: { composition: typeof composition };

declare let done: (compositionResult: CompositionResult) => void;
declare let serviceList: { sdl: string; name: string; url: string }[];

try {
  /**
   * @type {{ errors: Error[], supergraphSdl?: undefined } | { errors?: undefined, supergraphSdl: string; }}
   */
  const composed = composition_bridge.composition(serviceList);
  done(
    composed.errors ? { Err: composed.errors } : { Ok: composed.supergraphSdl }
  );
} catch (err) {
  done({ Err: [err] });
}
