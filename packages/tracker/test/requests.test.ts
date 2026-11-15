// verifies: tracker request behavior (batching, first event immediate, flush on hide)
// Runs the real tracker in a vm sandbox with a fake clock and counts beacon requests.
import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';
import { decode } from '@kurzhaar/protocol';
import { loadRegistry } from '@kurzhaar/protocol/registry';
import { transformSync } from 'esbuild';

// The tracker is TypeScript; strip the types and run the result unminified.
const code = transformSync(readFileSync(new URL('../src/tracker.ts', import.meta.url), 'utf8'), { loader: 'ts' }).code;

type Handler = () => void;
interface Timer { id: number; at: number; f: Handler }
interface Lt { track(name: string, props?: Record<string, string>): void; identify(id: string): void; flush(): void }
interface Controls {
  advance(ms: number): void;
  go(path: string, title: string): void;
  hide(): void;
  query(q: string): void;
  lt: Lt;
}
type Scenario = (c: Controls) => void;

function run(attrs: Record<string, string>, scenario: Scenario) {
  let clock = 1731150000000, timers: Timer[] = [], id = 0;
  const bodies: string[] = [];
  const wl: Record<string, Handler> = {}, dl: Record<string, Handler> = {};
  const loc = { href: 'https://acme.com/', pathname: '/', origin: 'https://acme.com' };
  const w: Record<string, any> = {
    history: { pushState() {}, replaceState() {} },
    location: loc,
    navigator: { language: 'uk-UA', sendBeacon: (_u: string, b: string) => { bodies.push(b); return true; } },
    localStorage: { getItem: () => null },
    addEventListener: (t: string, f: Handler) => (wl[t] = f),
  };
  const d: Record<string, any> = {
    currentScript: { getAttribute: (k: string) => attrs[k] ?? null, src: 'https://cdn/lt.js' },
    documentElement: { clientWidth: 1920, clientHeight: 1080 },
    title: 'Home', referrer: '', visibilityState: 'visible',
    addEventListener: (t: string, f: Handler) => (dl[t] = f),
  };
  w.window = w;
  const setTimeout_ = (f: Handler, ms: number) => { timers.push({ id: ++id, at: clock + ms, f }); return id; };
  const clearTimeout_ = (i: number) => { timers = timers.filter((t) => t.id !== i); };
  const advance = (ms: number) => {
    const end = clock + ms;
    for (;;) {
      timers.sort((a, b) => a.at - b.at);
      const t = timers[0];
      if (!t || t.at > end) break;
      timers.shift(); clock = Math.max(clock, t.at); t.f();
    }
    clock = end;
  };
  vm.runInNewContext(code, { window: w, document: d, URL, URLSearchParams, setTimeout: setTimeout_, clearTimeout: clearTimeout_, Date: { now: () => clock }, JSON, fetch() {} });
  const go = (p: string, title: string) => { loc.pathname = p; loc.href = 'https://acme.com' + p; d.title = title; w.history.pushState({}, '', p); advance(1); };
  const hide = () => { d.visibilityState = 'hidden'; dl.visibilitychange && dl.visibilitychange(); w.lt.flush(); };
  const query = (q: string) => { loc.href = 'https://acme.com' + loc.pathname + q; w.history.replaceState({}, '', loc.href); advance(1); };
  scenario({ advance, go, hide, query, lt: w.lt });
  const events = bodies.reduce((a, b) => a + decode(b).events.length, 0);
  return { requests: bodies.length, events, bodies };
}

const spa: Scenario = ({ advance, go, hide, lt }) => {
  advance(10); go('/pricing', 'P'); advance(4000); go('/docs', 'D'); advance(5000);
  go('/api', 'A'); advance(6000); go('/signup', 'S'); advance(5000);
  lt.track('signup', { plan: 'pro' }); advance(3000); hide();
};

test('bounce sends exactly one request with one event', () => {
  const r = run({ 'data-key': 'k' }, ({ advance, hide }) => { advance(2000); hide(); });
  assert.deepEqual({ requests: r.requests, events: r.events }, { requests: 1, events: 1 });
});

test('batching off sends one request per event', () => {
  const r = run({ 'data-key': 'k', 'data-batch': 'false' }, spa);
  assert.equal(r.events, 6);
  assert.equal(r.requests, 6);
});

test('events spaced wider than the default 3 s window are not merged', () => {
  const r = run({ 'data-key': 'k' }, spa);
  assert.equal(r.events, 6);
  assert.equal(r.requests, 6);
});

test('a wider batch window merges spaced events into fewer requests', () => {
  const r = run({ 'data-key': 'k', 'data-batch-ms': '10000' }, spa);
  assert.equal(r.events, 6);
  assert.ok(r.requests < 6, `expected fewer than 6 requests, got ${r.requests}`);
});

test('a burst within the batch window is merged into two requests', () => {
  const r = run({ 'data-key': 'k' }, ({ advance, lt, hide }) => {
    advance(40); lt.track('cta'); advance(40); lt.identify('u1'); advance(100); hide();
  });
  assert.equal(r.events, 3);
  assert.equal(r.requests, 2);
});

test('query-only changes do not create a pageview', () => {
  const r = run({ 'data-key': 'k' }, ({ advance, query, hide }) => {
    advance(10); query('?tab=2'); advance(10); query('?tab=3'); advance(10); hide();
  });
  assert.equal(r.events, 1);
});

// verifies: REQ-PROTOCOL-87M3 (every key the tracker writes is in the field registry)
test('the tracker writes only keys that the registry lists', () => {
  const registered = new Set(loadRegistry().fields.map((f) => f.key));
  const r = run({ 'data-key': 'k' }, ({ advance, go, lt, hide }) => {
    advance(10); lt.track('cta', { plan: 'pro' }); go('/pricing', 'P'); lt.identify('u1'); advance(4000); hide();
  });
  assert.ok(r.bodies.length > 0);
  for (const body of r.bodies)
    for (const line of body.split('\n'))
      for (const part of line.split('|')) {
        const key = part.slice(0, part.indexOf('::'));
        assert.ok(registered.has(key) || (key.startsWith('p.') && registered.has('p.<name>')), `key "${key}" is not in contracts/protocol/fields.json`);
      }
});
