// verifies: REQ-PROTOCOL-87M3 (registry is well formed), REQ-PROTOCOL-J5CJ (the reference decoder knows every key), REQ-PROTOCOL-1EDH (generated reference is current)
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { decode } from '../src/decode.ts';
import { loadRegistry } from '../src/registry.ts';
import type { RegistryField } from '../src/registry.ts';
import { render } from '../scripts/reference.ts';

const reg = loadRegistry();

test('every entry is complete and keys are unique', () => {
  const seen = new Set<string>();
  for (const f of reg.fields) {
    assert.ok(!seen.has(f.key), `duplicate key ${f.key}`);
    seen.add(f.key);
    assert.match(f.since, /^\d+\.\d+$/, f.key);
    assert.ok(['header', 'event'].includes(f.scope), f.key);
    assert.ok(['string', 'int', 'decimal'].includes(f.type), f.key);
    assert.ok(f.description.length > 0 && f.example.length > 0, f.key);
    assert.equal(f.scope === 'header', f.types === null, `${f.key}: types is set exactly for event fields`);
  }
  const ys = reg.eventTypes.map((t) => t.y);
  assert.equal(new Set(ys).size, ys.length, 'duplicate event type');
});

test('every event field names only registered event types', () => {
  const ys = new Set(reg.eventTypes.map((t) => t.y));
  for (const f of reg.fields) if (Array.isArray(f.types)) for (const y of f.types) assert.ok(ys.has(y), `${f.key}: unknown type ${y}`);
});

/** A minimal batch in which the field under test is present once. */
function wire(f: RegistryField): string {
  const key = f.family ? `p.${f.exampleName}` : f.key;
  if (f.scope === 'header') return f.key === 'v' ? 'v::1\nts::1' : `v::1|${key}::${f.example}\nts::1`;
  if (f.key === 'ts') return `v::1\nts::${f.example}`;
  if (f.key === 'd') return `v::1\nts::1\nd::${f.example}`;
  const y = f.key === 'y' ? f.example : Array.isArray(f.types) ? String(f.types[0]) : '0';
  return `v::1\nts::1|y::${y}|${key}::${f.example}`.replace('|y::0', '');
}

for (const f of reg.fields) {
  test(`decoder knows key "${f.key}"`, () => {
    const out = decode(wire(f)) as unknown as Record<string, unknown> & { events: Record<string, unknown>[] };
    if (f.decoded === null) {
      // v and d are consumed while decoding: v must be accepted, d must move the clock.
      if (f.key === 'd') assert.equal(out.events[1]!.ts, 1 + Number(f.example));
      return;
    }
    const where = f.scope === 'header' ? out : out.events[0]!;
    const got = where[f.decoded];
    if (f.family) assert.equal((got as Record<string, string>)[f.exampleName!], f.example);
    else if (f.type === 'string') assert.equal(got, f.example);
    else assert.equal(got, Number(f.example));
  });
}

test('docs/protocol.md matches the registry', () => {
  const current = readFileSync(new URL('../../../docs/protocol.md', import.meta.url), 'utf8');
  assert.equal(current, render(), 'run `pnpm --filter @kurzhaar/protocol docs`');
});
