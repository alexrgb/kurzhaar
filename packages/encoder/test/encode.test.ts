import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { decode } from '@kurzhaar/protocol';
import { loadRegistry } from '@kurzhaar/protocol/registry';
import { encodeBatch, encodeEvents, encodeHeader, escapePropName, escapeValue } from '../src/index.ts';
import type { BatchHeader, EventType, WireEvent } from '../src/index.ts';

interface Decoded {
  key?: string;
  screen?: string;
  lang?: string;
  identity?: string;
  events: { ts: number; type: number; name?: string; url?: string; title?: string; referrer?: string; props: Record<string, string>; engagedMs?: number; scrollPct?: number; lcp?: number; inp?: number; cls?: number; fcp?: number; ttfb?: number }[];
}
interface Vector {
  name: string;
  body: string;
  expected?: Decoded;
  error?: string;
}
const { vectors } = JSON.parse(readFileSync(new URL('../../../contracts/protocol/vectors.json', import.meta.url), 'utf8')) as {
  vectors: Vector[];
};

function encodeDecoded(d: Decoded): string {
  const header: BatchHeader = { key: d.key, screen: d.screen, lang: d.lang, identity: d.identity };
  const events: WireEvent[] = d.events.map((e) => ({ ...e, type: e.type as EventType }));
  return encodeBatch(header, events);
}
const canonical = (body: string): Decoded => JSON.parse(JSON.stringify(decode(body)));

// Vectors that only test decoder behaviour. They are not encoder inputs:
//  - "unsupported version": no valid expected output (the encoder always writes v::1).
//  - "unknown fields are ignored": the body contains a field the encoder never writes, so
//    it can not be reproduced byte for byte; it still round-trips semantically below.
const NOT_BYTE_EXACT = new Set(['unknown fields are ignored']);

for (const v of vectors) {
  if (v.error || !v.expected) {
    test(`vector "${v.name}" is decode-only (error vector, skipped for encoding)`, () => {
      assert.ok(v.error);
    });
    continue;
  }
  const expected = v.expected;
  // verifies: REQ-PROTOCOL-6NJT#S1 (shared vectors also exercise the TypeScript encoder)
  test(`vector "${v.name}": decode(encode(expected)) equals expected`, () => {
    assert.deepEqual(canonical(encodeDecoded(expected)), expected);
  });
  if (!NOT_BYTE_EXACT.has(v.name)) {
    test(`vector "${v.name}": encode(expected) reproduces body byte for byte`, () => {
      assert.equal(encodeDecoded(expected), v.body);
    });
  }
}

// verifies: REQ-PROTOCOL-16FY#S1
test('header carries the version as its first field', () => {
  assert.equal(encodeHeader(), 'v::1');
  assert.ok(encodeHeader({ key: 'abc', identity: 'u1' }).startsWith('v::1|'));
  assert.ok(encodeBatch({ key: 'abc' }, [{ ts: 1 }]).startsWith('v::1|k::abc\n'));
});

test('header field order and omission of empty fields', () => {
  assert.equal(encodeHeader({ key: 'k', screen: '1x2', lang: 'en', identity: 'me' }), 'v::1|k::k|s::1x2|l::en|i::me');
  assert.equal(encodeHeader({ key: '', lang: undefined }), 'v::1');
});

// verifies: REQ-PROTOCOL-55QJ#S1
test('three events yield ts then deltas and the decoder restores absolute times', () => {
  const t = 1731150000000;
  const body = encodeBatch({ key: 'abc' }, [{ ts: t }, { ts: t + 4000 }, { ts: t + 4500 }]);
  const lines = body.split('\n');
  assert.deepEqual(lines.slice(1), [`ts::${t}`, 'd::4000', 'd::500']);
  assert.deepEqual(decode(body).events.map((e) => e.ts), [t, t + 4000, t + 4500]);
});

test('same-millisecond and out-of-order events keep their order (delta 0 and negative)', () => {
  const body = encodeBatch({}, [{ ts: 100 }, { ts: 100 }, { ts: 90 }]);
  assert.deepEqual(body.split('\n').slice(1), ['ts::100', 'd::0', 'd::-10']);
  assert.deepEqual(decode(body).events.map((e) => e.ts), [100, 100, 90]);
});

test('rejects non-integer timestamps', () => {
  assert.throws(() => encodeEvents([{ ts: 1.5 }]), RangeError);
  assert.throws(() => encodeEvents([{ ts: Number.NaN }]), RangeError);
});

test('no events: header only', () => {
  assert.equal(encodeBatch({ key: 'abc' }, []), 'v::1|k::abc');
  assert.deepEqual(encodeEvents([]), []);
});

test('event type 0 is omitted, 1 and 2 are written', () => {
  const [a, b, c] = encodeEvents([{ ts: 1, type: 0 }, { ts: 2, type: 1 }, { ts: 3, type: 2 }]);
  assert.equal(a, 'ts::1');
  assert.equal(b, 'd::1|y::1');
  assert.equal(c, 'd::1|y::2');
});

test('property values: booleans as 1/0, numbers, objects as JSON, empties dropped', () => {
  const [line] = encodeEvents([
    { ts: 1, type: 1, name: 'x', props: { yes: true, no: false, n: 0, f: 1.5, o: { a: [1, '|'] }, nil: null, und: undefined, empty: '' } },
  ]);
  assert.equal(line, 'ts::1|y::1|n::x|p.yes::1|p.no::0|p.n::0|p.f::1.5|p.o::{"a":[1,"%7C"]}');
  assert.deepEqual(decode('v::1\n' + line).events[0]?.props, { yes: '1', no: '0', n: '0', f: '1.5', o: '{"a":[1,"|"]}' });
});

test('escape helpers', () => {
  assert.equal(escapeValue('a%b|c\rd\ne:f'), 'a%25b%7Cc%0Dd%0Ae:f');
  assert.equal(escapePropName('a:b%c'), 'a%3Ab%25c');
});

// verifies: REQ-PROTOCOL-9B66#S1
test('values with pipe, newline, CR, percent and escape lookalikes round-trip without extra fields or events', () => {
  const nasty = ['a|b\nc', 'x\r\ny', '100%', '%7C', '%25', '::', 'k::v|z::1', 'ünï 😀', '%'];
  for (const s of nasty) {
    const body = encodeBatch({ key: s, identity: s }, [
      { ts: 5, type: 1, name: s, url: s, title: s, referrer: s, props: { [s]: s, plain: s } },
    ]);
    assert.equal(body.split('\n').length, 2, `one header + one event for ${JSON.stringify(s)}`);
    const d = decode(body);
    assert.equal(d.key, s);
    assert.equal(d.identity, s);
    assert.equal(d.events.length, 1);
    const e = d.events[0];
    assert.deepEqual([e?.name, e?.url, e?.title, e?.referrer], [s, s, s, s]);
    assert.deepEqual(e?.props, { [s]: s, plain: s });
  }
});

test('property names containing ":" and "::" round-trip', () => {
  const body = encodeBatch({}, [{ ts: 1, props: { 'a:b': '1', 'c::d': '2', ':': '3' } }]);
  assert.deepEqual(decode(body).events[0]?.props, { 'a:b': '1', 'c::d': '2', ':': '3' });
});

// verifies: REQ-PROTOCOL-J5CJ (the encoder writes only registered keys and can write every registered key)

const registry = loadRegistry();
const registered = new Set(registry.fields.map((f) => f.key));
const allowed = (key: string) => registered.has(key) || (key.startsWith('p.') && registered.has('p.<name>'));
const keysOf = (line: string) => line.split('|').map((p) => p.slice(0, p.indexOf('::')));

test('encoder writes only keys that the registry lists', () => {
  const full = {
    ts: 1, name: 'n', url: '/', title: 't', referrer: 'r', engagedMs: 1, scrollPct: 1, lcp: 1, inp: 1, cls: 0.1, fcp: 1, ttfb: 1, props: { a: 'b' },
  };
  const body = encodeBatch(
    { key: 'k', screen: '1x1', lang: 'en', identity: 'i' },
    [full, { ...full, ts: 2, type: 1 as EventType }, { ...full, ts: 3, type: 3 as EventType }],
  );
  for (const line of body.split('\n')) for (const k of keysOf(line)) assert.ok(allowed(k), `key "${k}" is not in contracts/protocol/fields.json`);
});

test('the encoder can write every registered key', () => {
  const header: Record<string, string> = { k: 'key', s: 'screen', l: 'lang', i: 'identity' };
  for (const f of registry.fields.filter((x) => x.scope === 'header' && x.decoded)) {
    const opt = header[f.key];
    assert.ok(opt, `no encoder option for header key ${f.key}`);
    assert.ok(encodeHeader({ [opt]: f.example } as BatchHeader).includes(`${f.key}::`), f.key);
  }
  for (const f of registry.fields.filter((x) => x.scope === 'event' && x.decoded && !x.family && x.key !== 'ts')) {
    const type = (Array.isArray(f.types) ? f.types[0] : 0) as EventType;
    const value = f.type === 'string' ? f.example : Number(f.example);
    const line = encodeEvents([{ ts: 1 }, { ts: 2, type, [f.decoded!]: value } as WireEvent])[1]!;
    assert.ok(line.includes(`${f.key}::`), `${f.key} is not written for ${f.decoded}`);
  }
});
