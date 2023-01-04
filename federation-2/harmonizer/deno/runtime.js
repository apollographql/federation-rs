function done(result) {
  Deno.core.ops.op_composition_result(result);
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
