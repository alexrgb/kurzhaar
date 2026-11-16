// verifies: REQ-TRACKER-YDD6 (the gzipped tracker with every collector stays within its budget)
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { gzipSync } from 'node:zlib';
import { transformSync } from 'esbuild';

export const BUDGET_BYTES = 4096;

test(`the gzipped tracker is at most ${BUDGET_BYTES} bytes`, () => {
  const source = readFileSync(new URL('../src/tracker.ts', import.meta.url), 'utf8');
  const { code } = transformSync(source, { loader: 'ts', minify: true, define: { __VERSION__: '"1.0.0"' } });
  const size = gzipSync(code, { level: 9 }).length;
  assert.ok(size <= BUDGET_BYTES, `tracker is ${size} bytes gzipped, the budget is ${BUDGET_BYTES} bytes (ADR-9G74)`);
});
