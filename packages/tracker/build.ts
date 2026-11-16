import { readFileSync } from 'node:fs';
import { build } from 'esbuild';

const { version } = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8')) as { version: string };

await build({
  entryPoints: ['src/tracker.ts'],
  outfile: 'dist/lt.js',
  minify: true,
  define: { __VERSION__: JSON.stringify(version) },
  bundle: false,
});
