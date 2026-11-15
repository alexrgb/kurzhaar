// verifies: REQ-METRICS-9N0K Golden dataset (identical numbers in the api and dashboard databases)
// verifies: REQ-METRICS-63R7 Redelivery (the same batch twice counts once)
// Needs a ClickHouse server: set CLICKHOUSE_URL (e.g. http://127.0.0.1:8123). Skipped otherwise.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { ClickHouse, fromEnv } from '../src/clickhouse.ts';
import { migrate } from '../src/migrate.ts';
import { SCHEMA_DIR, loadTargets, renderQuery } from '../src/render.ts';

const skip = process.env.CLICKHOUSE_URL ? false : 'CLICKHOUSE_URL is not set';
const targets = loadTargets();
const dir = join(SCHEMA_DIR, '..', 'spec', 'golden', 'basic');
const events = readFileSync(join(dir, 'events.ndjson'), 'utf8').trim().split('\n').map((l) => JSON.parse(l) as object);
const expected = JSON.parse(readFileSync(join(dir, 'expected.json'), 'utf8')) as {
  rawEventCount: number;
  cases: { name: string; query: string; params: Record<string, string | number>; rows: Record<string, unknown>[] }[];
};

const DB = { raw: 'golden_raw', api: 'golden_api', dashboard: 'golden_dashboard' } as const;
const ch = new ClickHouse(fromEnv());

async function fresh(): Promise<void> {
  for (const db of Object.values(DB)) await ch.exec(`DROP DATABASE IF EXISTS ${db} SYNC`);
  for (const t of ['raw', 'api', 'dashboard'] as const) await migrate(ch, targets, t, DB[t]);
}

async function count(db: string): Promise<number> {
  const [r] = await ch.query<{ n: number }>(`SELECT count() AS n FROM ${db}.events`);
  return r!.n;
}

test('migrate is idempotent and refuses a changed migration', { skip }, async () => {
  await fresh();
  const again = await migrate(ch, targets, 'api', DB.api);
  assert.deepEqual(again.applied, []);
  assert.equal(again.skipped.length, targets.api.templates.length);
  await ch.exec(`ALTER TABLE ${DB.api}.schema_migrations UPDATE checksum = 'x' WHERE name = 'events' SETTINGS mutations_sync = 2`);
  await assert.rejects(migrate(ch, targets, 'api', DB.api), /changed after it was applied/);
});

test('golden dataset gives the expected numbers in both api and dashboard databases', { skip }, async () => {
  await fresh();
  for (const t of ['raw', 'api', 'dashboard'] as const) await ch.insert(`${DB[t]}.events`, events, 'golden-load');
  assert.equal(await count(DB.raw), expected.rawEventCount);
  // The golden events are older than 30 days, so the api and dashboard TTL drops their raw copies
  // at once while the rollups keep the numbers. Only the rollups are compared there.
  for (const t of ['api', 'dashboard'] as const) {
    for (const c of expected.cases) {
      const sql = renderQuery(targets, t, c.query, DB[t]);
      const rows = await ch.query(sql, c.params);
      assert.deepEqual(rows, c.rows, `${t}: ${c.name}`);
    }
  }
});

test('the same batch inserted twice with one token is counted once; a new token counts again', { skip }, async () => {
  await fresh();
  const totals = async () => {
    const sql = renderQuery(targets, 'api', 'totals', DB.api);
    return ch.query<{ views: number }>(sql, { site: 'acme', from: '2026-03-01 00:00:00', to: '2026-03-03 00:00:00' });
  };
  await ch.insert(`${DB.api}.events`, events, 'batch-1');
  await ch.insert(`${DB.api}.events`, events, 'batch-1');
  assert.equal((await totals())[0]!.views, 10);
  await ch.insert(`${DB.api}.events`, events, 'batch-2');
  assert.equal((await totals())[0]!.views, 20);
});
