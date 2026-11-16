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
  show(): void;
  scroll(y: number): void;
  /** Delivers performance entries of one type to the observers watching it. */
  emit(type: string, entries: Record<string, unknown>[]): void;
  query(q: string): void;
  lt: Lt;
}
type Scenario = (c: Controls) => void;

function run(attrs: Record<string, string>, scenario: Scenario, opts: { href?: string; tzMinutesBehindUtc?: number; hints?: { delayMs: number; platformVersion?: string }; touch?: number; coarse?: boolean; docHeight?: number; viewport?: number; nav?: { responseStart: number }; noPerf?: boolean } = {}) {
  let clock = 1731150000000, timers: Timer[] = [], id = 0;
  const bodies: string[] = [];
  const wl: Record<string, Handler> = {}, dl: Record<string, Handler> = {};
  const start = new URL(opts.href ?? 'https://acme.com/');
  const loc = { href: start.href, pathname: start.pathname, origin: start.origin };
  const observers: { type: string; cb: (l: { getEntries(): unknown[] }) => void }[] = [];
  class FakePO {
    cb: (l: { getEntries(): unknown[] }) => void;
    constructor(cb: (l: { getEntries(): unknown[] }) => void) { this.cb = cb; }
    observe(o: { type: string }) { observers.push({ type: o.type, cb: this.cb }); }
  }
  const w: Record<string, any> = {
    pageYOffset: 0,
    PerformanceObserver: opts.noPerf ? undefined : FakePO,
    performance: { getEntriesByType: () => (opts.nav ? [opts.nav] : []) },
    history: { pushState() {}, replaceState() {} },
    location: loc,
    navigator: { language: 'uk-UA', maxTouchPoints: opts.touch ?? 0, sendBeacon: (_u: string, b: string) => { bodies.push(b); return true; } },
    matchMedia: (q: string) => ({ matches: q === '(pointer:coarse)' && !!opts.coarse }),
    localStorage: { getItem: () => null },
    addEventListener: (t: string, f: Handler) => (wl[t] = f),
  };
  const d: Record<string, any> = {
    currentScript: { getAttribute: (k: string) => attrs[k] ?? null, src: 'https://cdn/lt.js' },
    documentElement: { clientWidth: 1920, clientHeight: opts.viewport ?? 1080, scrollHeight: opts.docHeight ?? 1080 },
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
  const show = () => { d.visibilityState = 'visible'; dl.visibilitychange && dl.visibilitychange(); };
  const scroll = (y: number) => { w.pageYOffset = y; wl.scroll && wl.scroll(); };
  const emit = (type: string, entries: Record<string, unknown>[]) => { for (const o of observers) if (o.type === type) o.cb({ getEntries: () => entries }); };
  const query = (q: string) => { loc.href = 'https://acme.com' + loc.pathname + q; w.history.replaceState({}, '', loc.href); advance(1); };
  scenario({ advance, go, hide, show, scroll, emit, query, lt: w.lt });
  // Counts pageviews and custom events; engagement and performance events are asserted on directly.
  const events = bodies.reduce((a, b) => a + decode(b).events.filter((e) => e.type < 3).length, 0);
  return { requests: bodies.length, events, bodies };
}

// The batching tests are about pageviews and custom events only, so the automatic ones are off.
const PLAIN = { 'data-key': 'k', 'data-engagement': 'false', 'data-vitals': 'false' };

const spa: Scenario = ({ advance, go, hide, lt }) => {
  advance(10); go('/pricing', 'P'); advance(4000); go('/docs', 'D'); advance(5000);
  go('/api', 'A'); advance(6000); go('/signup', 'S'); advance(5000);
  lt.track('signup', { plan: 'pro' }); advance(3000); hide();
};

test('bounce sends exactly one request with one event', () => {
  const r = run(PLAIN, ({ advance, hide }) => { advance(2000); hide(); });
  assert.deepEqual({ requests: r.requests, events: r.events }, { requests: 1, events: 1 });
});

test('batching off sends one request per event', () => {
  const r = run({ ...PLAIN, 'data-batch': 'false' }, spa);
  assert.equal(r.events, 6);
  assert.equal(r.requests, 6);
});

test('events spaced wider than the default 3 s window are not merged', () => {
  const r = run(PLAIN, spa);
  assert.equal(r.events, 6);
  assert.equal(r.requests, 6);
});

test('a wider batch window merges spaced events into fewer requests', () => {
  const r = run({ ...PLAIN, 'data-batch-ms': '10000' }, spa);
  assert.equal(r.events, 6);
  assert.ok(r.requests < 6, `expected fewer than 6 requests, got ${r.requests}`);
});

test('a burst within the batch window is merged into two requests', () => {
  const r = run(PLAIN, ({ advance, lt, hide }) => {
    advance(40); lt.track('cta'); advance(40); lt.identify('u1'); advance(100); hide();
  });
  assert.equal(r.events, 3);
  assert.equal(r.requests, 2);
});

test('query-only changes do not create a pageview', () => {
  const r = run(PLAIN, ({ advance, query, hide }) => {
    advance(10); query('?tab=2'); advance(10); query('?tab=3'); advance(10); hide();
  });
  assert.equal(r.events, 1);
});

// verifies: REQ-TRACKER-8PXD#S1 (a bounce is the pageview at once and the engagement event when the page is left)
test('with engagement on, a bounce is two requests: the pageview now and the engagement at leave', () => {
  const r = run({ 'data-key': 'k' }, ({ advance, hide }) => { advance(2000); hide(); });
  assert.deepEqual(r.bodies.map((b) => decode(b).events.map((e) => e.type)), [[0], [3]]);
});

// verifies: REQ-PROTOCOL-87M3 (every key the tracker writes is in the field registry)
test('the tracker writes only keys that the registry lists', () => {
  const registered = new Set(loadRegistry().fields.map((f) => f.key));
  const r = run({ 'data-key': 'k' }, ({ advance, go, lt, hide, emit }) => {
    emit('paint', [{ name: 'first-contentful-paint', startTime: 500 }]); emit('largest-contentful-paint', [{ startTime: 900 }]);
    emit('layout-shift', [{ startTime: 1, value: 0.1 }]); emit('event', [{ startTime: 1, duration: 80, interactionId: 1 }]);
    advance(10); lt.track('cta', { plan: 'pro' }, ['a', 'b']); go('/pricing', 'P'); lt.identify('u1'); advance(4000); hide();
  });
  assert.ok(r.bodies.length > 0);
  const seen = new Set(r.bodies.flatMap((b) => b.split('\n').flatMap((l) => l.split('|').map((p) => p.slice(0, p.indexOf('::'))))));
  for (const k of ['et', 'sc', 'lcp', 'inp', 'cls', 'fcp', 'g', 'tv', 'tz', 'cb', 'tp', 'pc']) assert.ok(seen.has(k) || k === 'cb', `the tracker never wrote ${k}`);
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

const all = (bodies: string[]) => bodies.flatMap((b) => decode(b).events);

// verifies: REQ-TRACKER-N7SJ#S1 (scrolled and left)
test('engagement carries the visible time and the maximum scroll depth when the page is hidden', () => {
  const r = run({ 'data-key': 'k' }, ({ advance, scroll, hide }) => {
    advance(5); scroll(0); advance(10000); scroll(1500); advance(10000); scroll(300); hide();
  }, { docHeight: 4000, viewport: 1000 });
  const e = all(r.bodies).filter((x) => x.type === 3);
  assert.equal(e.length, 1);
  assert.equal(e[0]!.engagedMs, 20005);
  assert.equal(e[0]!.scrollPct, 63); // (1500 + 1000) / 4000
  assert.equal(e[0]!.url, '/');
});

// verifies: REQ-TRACKER-N7SJ#S2 (single-page navigation sends engagement for the page that was left)
test('engagement for the old path is sent on a single-page navigation and counting restarts', () => {
  const r = run({ 'data-key': 'k' }, ({ advance, go, hide }) => {
    advance(5000); go('/b', 'B'); advance(3000); hide();
  });
  const e = all(r.bodies).filter((x) => x.type === 3);
  assert.deepEqual(e.map((x) => [x.url, x.engagedMs]), [['/', 5000], ['/b', 3001]]);
});

// verifies: REQ-TRACKER-N7SJ#S3 (tab left and revisited)
test('hiding and showing the tab produces two additive engagement events', () => {
  const r = run({ 'data-key': 'k' }, ({ advance, hide, show }) => {
    advance(10000); hide(); advance(60000); show(); advance(5000); hide();
  });
  assert.deepEqual(all(r.bodies).filter((x) => x.type === 3).map((x) => x.engagedMs), [10000, 5000]);
});

// verifies: REQ-TRACKER-N7SJ#S4 (a page that fits in one screen reports 100)
test('a short page reports a scroll depth of 100', () => {
  const r = run({ 'data-key': 'k' }, ({ advance, hide }) => { advance(2000); hide(); });
  assert.equal(all(r.bodies).find((x) => x.type === 3)?.scrollPct, 100);
});

// verifies: REQ-TRACKER-4EHV#S2 (engagement can be switched off)
test('data-engagement=false sends no engagement event', () => {
  const r = run({ 'data-key': 'k', 'data-engagement': 'false' }, ({ advance, go, hide }) => { advance(2000); go('/b', 'B'); advance(2000); hide(); });
  assert.equal(all(r.bodies).filter((x) => x.type === 3).length, 0);
});

// verifies: REQ-TRACKER-5AE5#S1 (one performance event with the available vitals)
test('one performance event for the initial page load carries the vitals', () => {
  const r = run({ 'data-key': 'k' }, ({ advance, emit, hide }) => {
    emit('paint', [{ name: 'first-paint', startTime: 100 }, { name: 'first-contentful-paint', startTime: 899.6 }]);
    emit('largest-contentful-paint', [{ startTime: 1200 }, { startTime: 1800.4 }]);
    emit('layout-shift', [{ startTime: 500, value: 0.03, hadRecentInput: false }, { startTime: 700, value: 0.02, hadRecentInput: false }, { startTime: 900, value: 0.5, hadRecentInput: true }]);
    emit('event', [{ startTime: 2000, duration: 48, interactionId: 1 }, { startTime: 3000, duration: 120, interactionId: 2 }, { startTime: 3500, duration: 400, interactionId: 0 }]);
    advance(5000); hide();
  }, { nav: { responseStart: 200.4 } });
  const p = all(r.bodies).filter((x) => x.type === 4);
  assert.equal(p.length, 1);
  assert.deepEqual([p[0]!.fcp, p[0]!.lcp, p[0]!.cls, p[0]!.inp, p[0]!.ttfb, p[0]!.url], [900, 1800, 0.05, 120, 200, '/']);
});

test('layout shifts more than 1 second apart start a new window and the largest window is CLS', () => {
  const r = run({ 'data-key': 'k' }, ({ emit, hide }) => {
    emit('layout-shift', [{ startTime: 100, value: 0.1 }, { startTime: 2000, value: 0.04 }, { startTime: 2500, value: 0.03 }]);
    hide();
  });
  assert.equal(all(r.bodies).find((x) => x.type === 4)?.cls, 0.1);
});

// verifies: REQ-TRACKER-5AE5#S2 (unsupported metric is left out)
test('a missing metric is omitted from the performance event', () => {
  const r = run({ 'data-key': 'k' }, ({ emit, hide }) => { emit('paint', [{ name: 'first-contentful-paint', startTime: 500 }]); hide(); });
  const p = all(r.bodies).find((x) => x.type === 4)!;
  assert.equal(p.fcp, 500);
  assert.ok(!('lcp' in p) && !('inp' in p) && !('cls' in p) && !('ttfb' in p));
});

// verifies: REQ-TRACKER-5AE5#S3 (no values: no performance event)
test('no performance event when the browser provides nothing', () => {
  const none = run({ 'data-key': 'k' }, ({ hide }) => hide(), { noPerf: true });
  assert.equal(all(none.bodies).filter((x) => x.type === 4).length, 0);
  const idle = run({ 'data-key': 'k' }, ({ hide }) => hide());
  assert.equal(all(idle.bodies).filter((x) => x.type === 4).length, 0);
});

// verifies: REQ-TRACKER-5AE5#S4 (single-page navigation sends no performance event; the initial load sends one)
test('the performance event is sent once, for the entry path, even after navigations', () => {
  const r = run({ 'data-key': 'k' }, ({ advance, go, emit, hide, show }) => {
    emit('paint', [{ name: 'first-contentful-paint', startTime: 400 }]);
    advance(1000); go('/b', 'B'); advance(1000); hide(); show(); advance(1000); hide();
  });
  const p = all(r.bodies).filter((x) => x.type === 4);
  assert.equal(p.length, 1);
  assert.equal(p[0]!.url, '/');
});

// verifies: REQ-TRACKER-4EHV#S1 (vitals can be switched off)
test('data-vitals=false sends no performance event', () => {
  const r = run({ 'data-key': 'k', 'data-vitals': 'false' }, ({ emit, hide }) => { emit('paint', [{ name: 'first-contentful-paint', startTime: 400 }]); hide(); });
  assert.equal(all(r.bodies).filter((x) => x.type === 4).length, 0);
});
