#!/usr/bin/env node
import { ClickHouse, fromEnv } from './clickhouse.ts';
import { migrate } from './migrate.ts';
import { loadTargets, render, type Targets } from './render.ts';

const usage = `usage:
  kurzhaar-schema render  <raw|api|dashboard> [--database NAME]   print the SQL
  kurzhaar-schema migrate <raw|api|dashboard> [--database NAME]   apply it (CLICKHOUSE_URL, CLICKHOUSE_USER, CLICKHOUSE_PASSWORD)`;

const [cmd, target, ...rest] = process.argv.slice(2);
const targets = loadTargets();
const i = rest.indexOf('--database');
const database = i >= 0 ? rest[i + 1] : undefined;

if (!target || !(target in targets) || (cmd !== 'render' && cmd !== 'migrate')) {
  console.error(usage);
  process.exit(2);
}
const t = target as keyof Targets;

if (cmd === 'render') {
  for (const m of render(targets, t, database)) console.log(`-- ${m.name}\n${m.statements.join(';\n\n')};\n`);
} else {
  const r = await migrate(new ClickHouse(fromEnv()), targets, t, database);
  console.log(`${t}: applied [${r.applied.join(', ')}], already applied [${r.skipped.join(', ')}]`);
}
