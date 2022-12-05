logger = {
    trace: (message) => Deno.core.opSync("log_trace", `${message.toString()}\n`),
    debug: (message) => Deno.core.opSync("log_debug", `${message.toString()}\n`),
    info: (message) => Deno.core.opSync("log_info", `${message.toString()}\n`),
    warn: (message) => Deno.core.opSync("log_warn", `${message.toString()}\n`),
    error: (message) => Deno.core.opSync("log_error", `${message.toString()}\n`),
};
function print(value) {
    Deno.core.print(`${value.toString()}\n`);
}
function done(result) {
    Deno.core.opSync("op_composition_result", result);
}
process = { env: { NODE_ENV: "production" }, argv: [] };
global = {};
exports = {};
//# sourceMappingURL=runtime.js.map