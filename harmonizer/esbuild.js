let denoFsPlugin = {
  name: 'fs',
  setup(build) {
    // Intercept require("fs") and replace it with shims
    build.onResolve({ filter: /^fs$/ }, args => ({
      path: args.path,
      namespace: 'deno-fs',
    }))

  build.onLoad({ filter: /.*/, namespace: 'deno-fs'}, () => ({
    contents: 'module.exports = { existsSync: Deno.ensureFileSync, writeFileSync: Deno.writeTextFile }',
    loader: 'js',
  }))
  }
}

require('esbuild').build({
  entryPoints: ['./js/index.mjs'],
  bundle: true,
  minify: true,
  sourcemap: true,
  target: 'es2020',
  globalName: 'composition',
  outfile: "./dist/composition.js",
  format: 'iife',
  plugins: [denoFsPlugin],
  define: { 'Buffer': 'dummy_buffer' },
  inject: ['./js/dummy_buffer.js']
}).catch(() => process.exit(1))
