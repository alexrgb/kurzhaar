// Generates docs/protocol.md from contracts/protocol/fields.json (ADR-VJGJ).
//   node scripts/reference.ts          write the file
//   node scripts/reference.ts --check  exit 1 when the committed file differs
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { loadRegistry } from '../src/registry.ts';

const OUT = new URL('../../../docs/protocol.md', import.meta.url);

export function render(): string {
  const r = loadRegistry();
  const cell = (s: string) => s.replace(/\|/g, '\\|');
  const types = (t: 'all' | number[] | null) => (t === null ? 'header' : t === 'all' ? 'all' : t.join(', '));
  const lines: string[] = [
    '# Wire format reference',
    '',
    '<!-- Generated from contracts/protocol/fields.json by packages/protocol/scripts/reference.ts. Do not edit. -->',
    '',
    `Wire format version ${r.protocol}, revision ${r.revision}. Revisions inside a version only add optional fields; decoders ignore keys they do not know. The decisions behind the format are in ADR-T7N1 and ADR-VJGJ.`,
    '',
    'A batch is plain text. Line 1 is the header, every following line is one event. A line is a list of `key::value` fields joined by `|`. In values, `%`, `|`, CR and LF are escaped as `%XX`.',
    '',
    '## Event types',
    '',
    '| `y` | Name | Status | Since | Description |',
    '|---|---|---|---|---|',
    ...r.eventTypes.map((t) => `| ${t.y} | ${t.name} | ${t.status} | ${t.since} | ${cell(t.description)} |`),
    '',
  ];
  for (const scope of ['header', 'event'] as const) {
    lines.push(scope === 'header' ? '## Header fields' : '## Event fields', '');
    lines.push('| Key | Name | Decoded as | Type | Unit | Required | Event types | Since | Description |');
    lines.push('|---|---|---|---|---|---|---|---|---|');
    for (const f of r.fields.filter((x) => x.scope === scope)) {
      lines.push(
        `| \`${f.key}\` | ${f.name} | ${f.decoded ? '`' + f.decoded + '`' : '-'} | ${f.type} | ${f.unit ?? '-'} | ${f.required ? 'yes' : 'no'} | ${scope === 'header' ? '-' : types(f.types)} | ${f.since} | ${cell(f.description)} |`,
      );
    }
    lines.push('');
  }
  lines.push('## Query parameters', '', 'Parameters of the collect request URL, not keys of the body. They never change the body, so the batch token stays the same.', '');
  lines.push('| Key | Name | Type | Unit | Range | Since | Description |', '|---|---|---|---|---|---|---|');
  for (const q of r.queryParameters) lines.push(`| \`${q.key}\` | ${q.name} | ${q.type} | ${q.unit ?? '-'} | ${q.min} to ${q.max} | ${q.since} | ${cell(q.description)} |`);
  lines.push('');
  lines.push('## Example', '', '```', 'v::1|k::abc|s::1920x1080|l::uk-UA|tv::1.4.2|tz::120', 'ts::1731150000000|u::/', 'd::4000|y::1|n::signup|g::paid,trial|p.plan::pro', 'd::8000|y::3|u::/|et::7500|sc::80', '```', '');
  return lines.join('\n');
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const text = render();
  if (process.argv.includes('--check')) {
    let current = '';
    try { current = readFileSync(OUT, 'utf8'); } catch {}
    if (current !== text) {
      console.error('docs/protocol.md is out of date; run `pnpm --filter @kurzhaar/protocol docs`');
      process.exit(1);
    }
  } else {
    writeFileSync(OUT, text);
  }
}
