let denoFsPlugin = {
  name: "fs",
  setup(build) {},
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
    inject: ["./esbuild/buffer_shim.js"],
  })
  .catch(() => process.exit(1));
