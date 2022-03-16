// all paths are relative to package.json when run with `npm run build`
require("esbuild")
  .build({
    entryPoints: ["./esbuild/apollo_federation_js.mjs"],
    bundle: true,
    minify: true,
    sourcemap: true,
    target: "es2020",
    globalName: "composition",
    outfile: "./dist/composition.js",
    format: "iife",
  })
  .catch(() => process.exit(1));
