const denoFsPlugin = {
  name: "fs",
  setup(build) {
    // Intercept require("fs") and replace it with shims
    build.onResolve({ filter: /^fs$/ }, (args) => ({
      path: args.path,
      namespace: "deno-fs",
    }));

    build.onLoad({ filter: /.*/, namespace: "deno-fs" }, () => ({
      resolveDir: ".",
      contents: `const { TextEncoder } = require("util");
module.exports = {
  existsSync(path) {
    try {
      Deno.statSync(path);
      return true;
    } catch (e) {
      if (e instanceof Deno.errors.NotFound) {
        return false;
      }
      throw e;
    }
  },
  writeFileSync(path, data, options) {
    return Deno.writeFileSync(
      path,
      typeof data === "string" ? new TextEncoder().encode(data) : data,
      options,
    );
  },
  readFileSync(path) {
    return Buffer.from(Deno.core.ops.op_read_bundled_file_sync(path));
  },
};`,
      loader: "js",
    }));
  },
};

// all paths are relative to package.json when run with `npm run build`
require("esbuild")
  .build({
    entryPoints: ["./js-dist/index.js"],
    bundle: true,
    minify: true,
    sourcemap: true,
    target: "es2020",
    globalName: "composition_bridge",
    outfile: "./bundled/composition_bridge.js",
    format: "iife",
    plugins: [denoFsPlugin],
    define: { Buffer: "buffer_shim" },
    inject: ["./esbuild/shims.js"],
  })
  .catch(() => process.exit(1));
