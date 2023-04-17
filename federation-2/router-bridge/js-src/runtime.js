// We define logging capabilities, which can be gathered by tracing
logger = {
  trace: (message) => Deno.core.ops.log_trace(`${message.toString()}\n`),
  debug: (message) => Deno.core.ops.log_debug(`${message.toString()}\n`),
  info: (message) => Deno.core.ops.log_info(`${message.toString()}\n`),
  warn: (message) => Deno.core.ops.log_warn(`${message.toString()}\n`),
  error: (message) => Deno.core.ops.log_error(`${message.toString()}\n`),
};

// We define a print function that uses
// Deno's print function to display the stringified argument.
function print(value) {
  Deno.core.print(`${value.toString()}\n`);
}

function done(result) {
  Deno.core.ops.deno_result(result);
}

crypto = {
  getRandomValues: (arg) => {
    Deno.core.ops.op_crypto_get_random_values(arg);
    return arg;
  },
};

// We build some of the preliminary objects that our Rollup-built package is
// expecting to be present in the environment.
// node_fetch_1 is an unused external dependency we don't bundle.  See the
// configuration in this package's 'rollup.config.js' for where this is marked
// as an external dependency and thus not packaged into the bundle.
node_fetch_1 = {};
// 'process' is a Node.js ism.  We rely on process.env.NODE_ENV, in
// particular, to determine whether or not we are running in a debug
// mode.  For the purposes of harmonizer, we don't gain anything from
// running in such a mode.
process = { argv: [], env: { NODE_ENV: "production" } };
// Some JS runtime implementation specific bits that we rely on that
// need to be initialized as empty objects.
global = {};
exports = {};
