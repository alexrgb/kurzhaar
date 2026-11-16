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
const code = transformSync(readFileSync(new URL('../src/tracker.ts', import.meta.url), 'utf8'), { loader: 'ts', define: { __VERSION__: '"1.4.2"' } }).code;

type Handler = () => void;
interface Timer { id: number; at: number; f: Handler }
interface Lt { track(name: string, props?: Record<string, string>, tags?: string[]): void; identify(id: string): void; flush(): void }
interface Controls {
  advance(ms: number): void;
  go(path: string, title: string): void;
  hide(): void;
  query(q: string): void;
  lt: Lt;
}
type Scenario = (c: Controls) => void;

function run(attrs: Record<string, string>, scenario: Scenario, opts: { href?: string; tzMinutesBehindUtc?: number; hints?: { delayMs: number; platformVersion?: string }; touch?: number; coarse?: boolean } = {}) {
  let clock = 1731150000000, timers: Timer[] = [], id = 0;
  const bodies: string[] = [];
  const wl: Record<string, Handler> = {}, dl: Record<string, Handler> = {};
  const start = new URL(opts.href ?? 'https://acme.com/');
  const loc = { href: start.href, pathname: start.pathname, origin: start.origin };
  const w: Record<string, any> = {
    history: { pushState() {}, replaceState() {} },
    location: loc,
    navigator: { language: 'uk-UA', maxTouchPoints: opts.touch ?? 0, sendBeacon: (_u: string, b: string) => { bodies.push(b); return true; } },
    matchMedia: (q: string) => ({ matches: q === '(pointer:coarse)' && !!opts.coarse }),
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
  if (opts.hints) {
    const hints = opts.hints;
    // A synchronous stand-in for the promise, so the fake clock decides when the answer arrives.
    w.navigator.userAgentData = {
      brands: [{ brand: 'Chromium', version: '124' }, { brand: 'Not/A)Brand', version: '99' }],
      mobile: false,
      platform: 'Windows',
      getHighEntropyValues: () => ({ then: (ok: (v: unknown) => void) => { setTimeout_(() => ok({ platformVersion: hints.platformVersion ?? '15.0.0' }), hints.delayMs); } }),
    };
  }
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
  vm.runInNewContext(code, { window: w, document: d, URL, URLSearchParams, setTimeout: setTimeout_, clearTimeout: clearTimeout_, Date: Object.assign(function () { return { getTimezoneOffset: () => opts.tzMinutesBehindUtc ?? 0 }; }, { now: () => clock }), JSON, fetch() {} });
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
    advance(10); lt.track('cta', { plan: 'pro' }, ['a', 'b']); go('/pricing', 'P'); lt.identify('u1'); advance(4000); hide();
  });
  assert.ok(r.bodies.length > 0);
  for (const body of r.bodies)
    for (const line of body.split('\n'))
      for (const part of line.split('|')) {
        const key = part.slice(0, part.indexOf('::'));
        assert.ok(registered.has(key) || (key.startsWith('p.') && registered.has('p.<name>')), `key "${key}" is not in contracts/protocol/fields.json`);
      }
});

// verifies: REQ-TRACKER-KE2V#S1 (a tagged event carries g)
test('tags are sent on the event', () => {
  const r = run({ 'data-key': 'k' }, ({ advance, lt, hide }) => {
    advance(10); lt.track('signup', { plan: 'pro' }, ['paid', 'trial']); hide();
  });
  const events = r.bodies.flatMap((b) => decode(b).events);
  assert.deepEqual(events.find((e) => e.name === 'signup')?.tags, ['paid', 'trial']);
  assert.equal(events.find((e) => e.type === 0)?.tags, undefined);
});

// verifies: REQ-TRACKER-KE2V#S2 (the header carries tv and tz, no zone name)
test('the header carries the tracker version and the offset in minutes east of UTC', () => {
  const r = run({ 'data-key': 'k' }, ({ advance, hide }) => { advance(10); hide(); }, { tzMinutesBehindUtc: 300 });
  assert.ok(r.bodies[0]!.split('\n')[0]!.includes('|tv::1.4.2|tz::-300'));
  const east = run({ 'data-key': 'k' }, ({ hide }) => hide(), { tzMinutesBehindUtc: -120 });
  assert.equal(decode(east.bodies[0]!).tzOffset, 120);
  const utc = run({ 'data-key': 'k' }, ({ hide }) => hide());
  assert.equal(decode(utc.bodies[0]!).tzOffset, 0);
  assert.equal(decode(utc.bodies[0]!).trackerVersion, '1.4.2');
});

// verifies: REQ-TRACKER-KE2V#S3 (at most 10 tags of 32 characters)
test('too many or too long tags are cut, empty tags and commas are handled', () => {
  const many = Array.from({ length: 12 }, (_, i) => 'tag' + i);
  const r = run({ 'data-key': 'k' }, ({ advance, lt, hide }) => {
    advance(10);
    lt.track('a', {}, many);
    lt.track('b', {}, ['x'.repeat(40), '', 'p,q']);
    hide();
  });
  const events = r.bodies.flatMap((b) => decode(b).events);
  assert.deepEqual(events.find((e) => e.name === 'a')?.tags, many.slice(0, 10));
  assert.deepEqual(events.find((e) => e.name === 'b')?.tags, ['x'.repeat(32), 'p,q']);
});

// verifies: REQ-TRACKER-50ST (the added click identifiers survive query stripping, other parameters do not)
test('click identifiers are kept in the URL and other parameters are stripped', () => {
  const href = 'https://acme.com/p?gclid=1&gad_source=2&dclid=3&wbraid=4&gbraid=5&mc_cid=6&igshid=7&token=secret';
  const r = run({ 'data-key': 'k' }, ({ hide }) => hide(), { href });
  const url = decode(r.bodies[0]!).events[0]?.url ?? '';
  for (const k of ['gclid', 'gad_source', 'dclid', 'wbraid', 'gbraid', 'mc_cid', 'igshid']) assert.ok(url.includes(k + '='), k);
  assert.ok(!url.includes('token'));
});

// verifies: REQ-COLLECT-BCES#S2 (no client hints: only touch points and pointer type)
test('without client hints the header carries only touch points and pointer type', () => {
  const r = run({ 'data-key': 'k' }, ({ hide }) => hide(), { touch: 5, coarse: true });
  const head = r.bodies[0]!.split('\n')[0]!;
  assert.ok(head.endsWith('|tp::5|pc::1'), head);
  assert.ok(!/\|(cb|cm|cp|cv)::/.test(head));
});

// verifies: REQ-COLLECT-BCES#S1, REQ-COLLECT-BCES#S3 (Chromium signals, platform major version only, no model or build)
test('with client hints the header carries brands, mobile flag, platform and the platform major version', () => {
  const r = run({ 'data-key': 'k' }, ({ advance, hide }) => { advance(50); hide(); }, { hints: { delayMs: 20, platformVersion: '15.0.0' } });
  const h = decode(r.bodies[0]!);
  assert.deepEqual(h.clientBrands, ['Chromium/124', 'Not/A)Brand/99']);
  assert.deepEqual([h.clientMobile, h.clientPlatform, h.clientPlatformVersion, h.touchPoints, h.coarsePointer], [0, 'Windows', '15', 0, 0]);
  assert.ok(!r.bodies[0]!.includes('15.0.0'));
});

// verifies: REQ-TRACKER-EZ0J#S1 (hints arrive in time: the first batch waits for them)
test('the first batch waits for a fast platform version', () => {
  const r = run({ 'data-key': 'k' }, ({ advance }) => { advance(10); }, { hints: { delayMs: 20 } });
  assert.equal(r.requests, 0);
  const done = run({ 'data-key': 'k' }, ({ advance }) => { advance(30); }, { hints: { delayMs: 20 } });
  assert.equal(done.requests, 1);
  assert.equal(decode(done.bodies[0]!).clientPlatformVersion, '15');
});

// verifies: REQ-TRACKER-EZ0J#S2 (slow hints: sent after 150 ms without cv, later batches carry it)
test('a slow platform version does not hold the first batch longer than 150 ms', () => {
  const slow = { hints: { delayMs: 200 } };
  assert.equal(run({ 'data-key': 'k' }, ({ advance }) => advance(149), slow).requests, 0);
  const sent = run({ 'data-key': 'k' }, ({ advance }) => advance(151), slow);
  assert.equal(sent.requests, 1);
  assert.ok(!/\|cv::/.test(sent.bodies[0]!.split('\n')[0]!), 'first batch has no cv');
  const later = run({ 'data-key': 'k' }, ({ advance, lt, hide }) => { advance(300); lt.track('late'); advance(10); hide(); }, slow);
  assert.equal(decode(later.bodies[later.bodies.length - 1]!).clientPlatformVersion, '15');
});

// verifies: REQ-TRACKER-EZ0J#S3 (no client hints in the browser: the first event goes at once)
test('without client hints the first event is sent at once', () => {
  const r = run({ 'data-key': 'k' }, () => {});
  assert.equal(r.requests, 1);
});
