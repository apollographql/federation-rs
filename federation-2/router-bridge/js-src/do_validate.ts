import type { validate } from ".";
import type { OperationResult } from "./types";

type JsError = {
  name: string;
  message: string;
  stack?: string;
  validationError?: boolean;
};

/**
 * There are several global properties that we make available in our V8 runtime
 * and these are the types for those that we expect to use within this script.
 * They'll be stripped in the emitting of this file as JS, of course.
 */
declare let bridge: { validate: typeof validate };

declare let done: (operationResult: OperationResult) => void;
declare let schema: string;
declare let query: string;

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

if (!schema) {
  done({
    Err: [{ message: "Error in JS-Rust-land: schema is empty." }],
  });
}

if (!query) {
  done({
    Err: [{ message: "Error in JS-Rust-land: query is empty." }],
  });
}
const diagnostics = bridge.validate(schema, query);
const diag = diagnostics.map((e) => intoSerializableError(e));

done({ Err: { errors: diag } });
// if (diagnostics.length > 0) {
//   done({ Err: [{ message: "there are diagnostics" }] });
// } else {
//   done({ Ok: "successfully validated" });
// }
