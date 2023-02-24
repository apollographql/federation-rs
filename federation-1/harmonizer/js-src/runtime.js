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

function done(result) {
  Deno.core.opAsync("op_composition_result", result);
}

// We build some of the preliminary objects that our esbuilt package is
// expecting to be present in the environment.
// 'process' is a Node.js ism. We rely on process.env.NODE_ENV, in
// particular, to determine whether or not we are running in a debug
// mode. For the purposes of harmonizer, we don't gain anything from
// running in such a mode.
process = { env: { NODE_ENV: "production" }, argv: [] };
// Some JS runtime implementation specific bits that we rely on that
// need to be initialized as empty objects.
global = {};
exports = {};
