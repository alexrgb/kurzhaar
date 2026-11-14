import { build } from 'esbuild';

await build({
  entryPoints: ['src/tracker.js'],
  outfile: 'dist/lt.js',
  minify: true,
  bundle: false,
});
