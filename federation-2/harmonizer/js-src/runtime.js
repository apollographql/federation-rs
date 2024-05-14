// We define logging capabilities, which can be gathered by tracing
logger = {
  trace: (message) => Deno.core.ops.log_trace(`${message.toString()}\n`),
  debug: (message) => Deno.core.ops.log_trace(`${message.toString()}\n`),
  info: (message) => Deno.core.ops.log_trace(`${message.toString()}\n`),
  warn: (message) => Deno.core.ops.log_trace(`${message.toString()}\n`),
  error: (message) => Deno.core.ops.log_trace(`${message.toString()}\n`),
};

// We define a print function that uses
// Deno's print function to display the stringified argument.
function print(value) {
  Deno.core.print(`${value.toString()}\n`);
}

// We build some of the preliminary objects that our esbuilt package is
// expecting to be present in the environment.
// 'process' is a Node.js ism. We rely on process.env.NODE_ENV, in
// particular, to determine whether or not we are running in a debug
// mode. For the purposes of harmonizer, we don't gain anything from
// running in such a mode.
process = { env: { NODE_ENV: "production" }, argv: [] };

// Since Deno does not have the __dirname variable of Node.js, we fake it in a
// way that is easy to spot (see op_read_bundled_file_sync in ../src/lib.rs).
__dirname = "<bundled>";

// Polyfill for the Node.js global object.
global = typeof globalThis === "object" ? globalThis : {};

// Needed only so Object.defineProperty(exports, "__esModule", { value: true })
// in do_compose.js doesn't throw a ReferenceError.
exports = {};
