// verifies: REQ-METRICS-FNX4 Template change (api and dashboard come from the same templates)
import test from 'node:test';
import assert from 'node:assert/strict';
import { loadTargets, render, splitStatements, substitute } from '../src/render.ts';

const targets = loadTargets();

test('api and dashboard render the same SQL apart from the database name', () => {
  const api = render(targets, 'api', 'DB').map((m) => m.statements);
  const dash = render(targets, 'dashboard', 'DB').map((m) => m.statements);
  assert.deepEqual(api, dash);
  assert.ok(api.length > 1);
});

test('api and dashboard have the same migration names and checksums for the same database name', () => {
  const key = (t: 'api' | 'dashboard') => render(targets, t, 'DB').map((m) => `${m.name}:${m.checksum}`);
  assert.deepEqual(key('api'), key('dashboard'));
});

test('raw has no TTL, api and dashboard keep 30 days', () => {
  const sql = (t: 'raw' | 'api' | 'dashboard') => render(targets, t)[0]!.statements.join('\n');
  assert.doesNotMatch(sql('raw'), /TTL/);
  assert.match(sql('api'), /TTL toDateTime\(ts\) \+ INTERVAL 30 DAY/);
  assert.match(sql('dashboard'), /TTL toDateTime\(ts\) \+ INTERVAL 30 DAY/);
});

test('every template renders without leftover placeholders', () => {
  for (const t of ['raw', 'api', 'dashboard'] as const) {
    for (const m of render(targets, t)) assert.doesNotMatch(m.statements.join('\n'), /\{\{/);
  }
});

test('an unknown placeholder is an error and ClickHouse parameters are left alone', () => {
  assert.throws(() => substitute('SELECT {{nope}}', {}), /unknown placeholder/);
  assert.equal(substitute('WHERE a = {site:String}', {}), 'WHERE a = {site:String}');
});

test('statements are split at line-ending semicolons and comments are dropped', () => {
  assert.deepEqual(splitStatements('-- c\nA;\n\nB;\n'), ['A', 'B']);
});
