import { build } from 'esbuild';

await build({
  entryPoints: ['src/tracker.ts'],
  outfile: 'dist/lt.js',
  minify: true,
  bundle: false,
});
