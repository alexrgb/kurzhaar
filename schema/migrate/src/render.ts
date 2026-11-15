import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
export const SCHEMA_DIR = join(here, '..', '..');
export const TEMPLATES_DIR = join(SCHEMA_DIR, 'templates');

/** Unique-visitor aggregate. Approximate by design (ADR S22Y); change here to change every rollup. */
export const UNIQ = 'uniqCombined64';

export interface TargetConfig {
  database: string;
  params: Record<string, string>;
  templates: string[];
}

export type Targets = Record<'raw' | 'api' | 'dashboard', TargetConfig>;

export function loadTargets(): Targets {
  return JSON.parse(readFileSync(join(SCHEMA_DIR, 'targets.json'), 'utf8')) as Targets;
}

/** Replace {{name}} placeholders. ClickHouse query parameters use single braces and are left alone. */
export function substitute(sql: string, values: Record<string, string>): string {
  return sql.replace(/\{\{(\w+)\}\}/g, (_m, name: string) => {
    const v = values[name];
    if (v === undefined) throw new Error(`unknown placeholder {{${name}}}`);
    return v;
  });
}

function valuesFor(target: TargetConfig, database?: string): Record<string, string> {
  return { database: database ?? target.database, uniq: UNIQ, ...target.params };
}

function readTemplate(relative: string): string {
  return readFileSync(join(TEMPLATES_DIR, relative), 'utf8');
}

/** Split a template into statements (a statement ends with a semicolon at the end of a line). */
export function splitStatements(sql: string): string[] {
  const cleaned = sql
    .split('\n')
    .filter((l) => !l.trimStart().startsWith('--'))
    .join('\n');
  return cleaned
    .split(/;\s*(?:\n|$)/)
    .map((s) => s.trim())
    .filter(Boolean);
}

export interface Migration {
  name: string;
  statements: string[];
  checksum: string;
}

/** Render the migrations of one target in order. One migration per template. */
export function render(targets: Targets, target: keyof Targets, database?: string): Migration[] {
  const cfg = targets[target];
  return cfg.templates.map((name) => {
    const statements = splitStatements(substitute(readTemplate(`${name}.sql`), valuesFor(cfg, database)));
    const checksum = createHash('sha256').update(statements.join(';\n')).digest('hex');
    return { name, statements, checksum };
  });
}

/** Render a read query (templates/queries/<name>.sql) for a target database. */
export function renderQuery(targets: Targets, target: keyof Targets, name: string, database?: string): string {
  return substitute(readTemplate(`queries/${name}.sql`), valuesFor(targets[target], database)).trim();
}
