import type { ClickHouse } from './clickhouse.ts';
import { render, type Targets } from './render.ts';

export interface Result {
  applied: string[];
  skipped: string[];
}

/**
 * Apply the migrations of a target to a database. Idempotent: a migration that is already recorded
 * with the same checksum is skipped. A recorded migration whose rendered SQL changed is an error:
 * a changed metric needs a new migration and a rollup rebuild from the RAW cluster (ADR S22Y).
 */
export async function migrate(ch: ClickHouse, targets: Targets, target: keyof Targets, database?: string): Promise<Result> {
  const db = database ?? targets[target].database;
  await ch.exec(`CREATE DATABASE IF NOT EXISTS ${db}`);
  await ch.exec(
    `CREATE TABLE IF NOT EXISTS ${db}.schema_migrations
     (name String, checksum String, applied_at DateTime DEFAULT now())
     ENGINE = MergeTree ORDER BY name`,
  );
  const known = new Map<string, string>();
  for (const r of await ch.query<{ name: string; checksum: string }>(`SELECT name, checksum FROM ${db}.schema_migrations`)) {
    known.set(r.name, r.checksum);
  }
  const result: Result = { applied: [], skipped: [] };
  for (const m of render(targets, target, db)) {
    const recorded = known.get(m.name);
    if (recorded === m.checksum) {
      result.skipped.push(m.name);
      continue;
    }
    if (recorded !== undefined) {
      throw new Error(`migration "${m.name}" changed after it was applied to ${db}; add a new migration and rebuild rollups from RAW`);
    }
    for (const s of m.statements) await ch.exec(s);
    await ch.insert(`${db}.schema_migrations`, [{ name: m.name, checksum: m.checksum }]);
    result.applied.push(m.name);
  }
  return result;
}
