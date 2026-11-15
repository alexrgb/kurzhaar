// verifies: tracker request behavior (batching, first event immediate, flush on hide)
// Runs the real tracker in a vm sandbox with a fake clock and counts beacon requests.
const test = require('node:test');
const assert = require('node:assert/strict');
const vm = require('node:vm');
const fs = require('node:fs');
const path = require('node:path');
const { decode } = require('@kurzhaar/protocol');

const { transformSync } = require('esbuild');

// The tracker is TypeScript; strip the types and run the result unminified.
const code = transformSync(fs.readFileSync(path.join(__dirname, '../src/tracker.ts'), 'utf8'), { loader: 'ts' }).code;

function run(attrs, scenario) {
  let clock = 1731150000000, timers = [], id = 0;
  const bodies = [], wl = {}, dl = {};
  const loc = { href: 'https://acme.com/', pathname: '/', origin: 'https://acme.com' };
  const w = {
    history: { pushState() {}, replaceState() {} },
    location: loc,
    navigator: { language: 'uk-UA', sendBeacon: (u, b) => { bodies.push(b); return true; } },
    localStorage: { getItem: () => null },
    addEventListener: (t, f) => (wl[t] = f),
  };
  const d = {
    currentScript: { getAttribute: (k) => attrs[k] ?? null, src: 'https://cdn/lt.js' },
    documentElement: { clientWidth: 1920, clientHeight: 1080 },
    title: 'Home', referrer: '', visibilityState: 'visible',
    addEventListener: (t, f) => (dl[t] = f),
  };
  w.window = w;
  const setTimeout_ = (f, ms) => { timers.push({ id: ++id, at: clock + ms, f }); return id; };
  const clearTimeout_ = (i) => { timers = timers.filter((t) => t.id !== i); };
  const advance = (ms) => {
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
  const go = (p, title) => { loc.pathname = p; loc.href = 'https://acme.com' + p; d.title = title; w.history.pushState({}, '', p); advance(1); };
  const hide = () => { d.visibilityState = 'hidden'; dl.visibilitychange && dl.visibilitychange(); w.lt.flush(); };
  const query = (q) => { loc.href = 'https://acme.com' + loc.pathname + q; w.history.replaceState({}, '', loc.href); advance(1); };
  scenario({ advance, go, hide, query, lt: w.lt });
  const events = bodies.reduce((a, b) => a + decode(b).events.length, 0);
  return { requests: bodies.length, events };
}

const spa = ({ advance, go, hide, lt }) => {
  advance(10); go('/pricing', 'P'); advance(4000); go('/docs', 'D'); advance(5000);
  go('/api', 'A'); advance(6000); go('/signup', 'S'); advance(5000);
  lt.track('signup', { plan: 'pro' }); advance(3000); hide();
};

test('bounce sends exactly one request with one event', () => {
  const r = run({ 'data-key': 'k' }, ({ advance, hide }) => { advance(2000); hide(); });
  assert.deepEqual(r, { requests: 1, events: 1 });
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
