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
/** What the fake collector says to one request: a status with an optional Retry-After, or a network error. */
type Answer = number | { status: number; retryAfter?: string } | 'error';
interface Call { via: 'fetch' | 'beacon'; url: string; body: string; at: number; init?: Record<string, unknown> }

function run(attrs: Record<string, string>, scenario: Scenario, opts: { href?: string; tzMinutesBehindUtc?: number; hints?: { delayMs: number; platformVersion?: string }; touch?: number; coarse?: boolean; docHeight?: number; viewport?: number; nav?: { responseStart: number }; noPerf?: boolean; answers?: Answer[]; random?: number; optOut?: () => boolean; noFetch?: boolean; noBeacon?: boolean } = {}) {
  let clock = 1731150000000, timers: Timer[] = [], id = 0;
  const bodies: string[] = [];
  const calls: Call[] = [];
  const storage: string[] = [];
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
    navigator: { language: 'uk-UA', maxTouchPoints: opts.touch ?? 0, sendBeacon: opts.noBeacon ? undefined : (u: string, b: string) => { bodies.push(b); calls.push({ via: 'beacon', url: u, body: b, at: clock }); return true; } },
    matchMedia: (q: string) => ({ matches: q === '(pointer:coarse)' && !!opts.coarse }),
    localStorage: { getItem: (k: string) => { storage.push('get:' + k); return opts.optOut && opts.optOut() ? '1' : null; }, setItem: (k: string) => { storage.push('set:' + k); } },
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
  // A synchronous stand-in for fetch: the answer is delivered when the tracker calls `then`, so the fake clock stays in charge.
  const answers = opts.answers ?? [];
  const fakeFetch = (u: string, init: Record<string, unknown>) => {
    const n = calls.filter((c) => c.via === 'fetch').length;
    calls.push({ via: 'fetch', url: u, body: init.body as string, at: clock, init });
    bodies.push(init.body as string);
    const a: Answer = answers[n] ?? 202;
    return {
      then: (ok: (r: unknown) => void, err: () => void) => {
        if (a === 'error') return err();
        const status = typeof a === 'number' ? a : a.status;
        const retryAfter = typeof a === 'number' ? null : a.retryAfter ?? null;
        ok({ status, headers: { get: (k: string) => (k === 'Retry-After' ? retryAfter : null) } });
      },
    };
  };
  vm.runInNewContext(code, { window: w, document: d, URL, URLSearchParams, setTimeout: setTimeout_, clearTimeout: clearTimeout_, Date: Object.assign(function () { return { getTimezoneOffset: () => opts.tzMinutesBehindUtc ?? 0 }; }, { now: () => clock }), JSON, Math: Object.assign(Object.create(Math), { random: () => opts.random ?? 0 }), ...(opts.noFetch ? {} : { fetch: fakeFetch }) });
  const go = (p: string, title: string) => { loc.pathname = p; loc.href = 'https://acme.com' + p; d.title = title; w.history.pushState({}, '', p); advance(1); };
  const hide = () => { d.visibilityState = 'hidden'; dl.visibilitychange && dl.visibilitychange(); w.lt.flush(); };
  const show = () => { d.visibilityState = 'visible'; dl.visibilitychange && dl.visibilitychange(); };
  const scroll = (y: number) => { w.pageYOffset = y; wl.scroll && wl.scroll(); };
  const emit = (type: string, entries: Record<string, unknown>[]) => { for (const o of observers) if (o.type === type) o.cb({ getEntries: () => entries }); };
  const query = (q: string) => { loc.href = 'https://acme.com' + loc.pathname + q; w.history.replaceState({}, '', loc.href); advance(1); };
  scenario({ advance, go, hide, show, scroll, emit, query, lt: w.lt });
  // Counts pageviews and custom events; engagement and performance events are asserted on directly.
  const events = bodies.reduce((a, b) => a + decode(b).events.filter((e) => e.type < 3).length, 0);
  return { requests: bodies.length, events, bodies, calls, storage };
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

const one: Scenario = ({ advance }) => { advance(10); };
const fetches = (r: { calls: Call[] }) => r.calls.filter((c) => c.via === 'fetch');
const ageOf = (c: Call) => Number(new URL(c.url).searchParams.get('a') ?? 0);

// verifies: REQ-TRACKER-7NVW.S1 (a normal send is fetch with keepalive, no credentials and a plain text body)
// verifies: REQ-TRACKER-7NVW.S4 (no custom header, so no preflight, with or without the age)
test('a normal send is a simple fetch with keepalive', () => {
  const r = run(PLAIN, one);
  const c = fetches(r);
  assert.equal(c.length, 1);
  assert.equal(r.calls.length, 1);
  assert.equal(c[0]!.init!.keepalive, true);
  assert.equal(c[0]!.init!.credentials, 'omit');
  assert.equal(c[0]!.init!.method, 'POST');
  assert.equal(c[0]!.init!.headers, undefined);
  assert.equal(c[0]!.url, 'https://cdn/v1/collect');
});

// verifies: REQ-TRACKER-7NVW.S2 (hidden page: every waiting batch goes once by beacon and nothing is retried)
test('hiding the page sends what waits by beacon and stops retrying', () => {
  const r = run(PLAIN, ({ advance, lt, hide }) => {
    advance(10);                // first batch goes by fetch and fails
    lt.track('a'); advance(10); // waits behind the failed one
    lt.track('b'); advance(10);
    hide();
    advance(600000);
  }, { answers: [503] });
  const beacons = r.calls.filter((c) => c.via === 'beacon');
  // One fetch that failed; at hide the failed batch and the one behind it each go once by beacon, and nothing is retried.
  assert.equal(fetches(r).length, 1);
  assert.equal(beacons.length, 2);
  assert.equal(beacons[0]!.body, fetches(r)[0]!.body);
  assert.deepEqual(decode(beacons[1]!.body).events.map((e) => e.name), ['a', 'b']);
  assert.equal(r.calls.length, 3);
});

// verifies: REQ-TRACKER-7NVW.S3 (no fetch: the beacon is used)
test('without fetch the beacon is used', () => {
  const r = run(PLAIN, one, { noFetch: true });
  assert.equal(r.calls.length, 1);
  assert.equal(r.calls[0]!.via, 'beacon');
});

// verifies: REQ-TRACKER-2P41.S1 (503 with Retry-After 5, then 202: the same body is sent after 5 s and then it stops)
// verifies: REQ-TRACKER-2P41.S8 (the second attempt carries the same bytes)
test('503 with Retry-After is retried with the same bytes and the age', () => {
  const r = run(PLAIN, ({ advance }) => { advance(10); advance(4900); advance(300); advance(60000); }, { answers: [{ status: 503, retryAfter: '5' }, 202] });
  const c = fetches(r);
  assert.equal(c.length, 2);
  assert.equal(c[0]!.body, c[1]!.body);
  assert.ok(c[1]!.at - c[0]!.at >= 5000);
  assert.ok(ageOf(c[1]!) >= 5000 && ageOf(c[1]!) < 5400, String(ageOf(c[1]!)));
  assert.equal(ageOf(c[0]!), 0);
});

// verifies: REQ-TRACKER-2P41.S2 (network errors: 1 s, 2 s, 4 s ... up to 60 s, with up to 50% jitter)
test('network errors back off from 1 s and double up to 60 s', () => {
  const errors: Answer[] = Array(12).fill('error');
  const r = run(PLAIN, ({ advance }) => { advance(10); advance(3600000 - 100); }, { answers: errors });
  const c = fetches(r);
  const gaps = c.slice(1).map((x, i) => x.at - c[i]!.at);
  assert.deepEqual(gaps.slice(0, 7), [1000, 2000, 4000, 8000, 16000, 32000, 60000]);
  assert.ok(gaps.slice(7).every((g) => g === 60000));
  // With the largest random draw each wait is 50% longer.
  const r2 = run(PLAIN, ({ advance }) => { advance(10); advance(200000); }, { answers: errors, random: 0.999999 });
  const c2 = fetches(r2);
  const g2 = c2.slice(1).map((x, i) => x.at - c2[i]!.at);
  assert.ok(g2[0]! > 1400 && g2[0]! <= 1500, String(g2[0]));
  assert.ok(g2[1]! > 2900 && g2[1]! <= 3000);
});

// verifies: REQ-TRACKER-2P41.S4 (429 with Retry-After 30: no attempt sooner than 30 s later)
test('429 honours a long Retry-After', () => {
  const r = run(PLAIN, ({ advance }) => { advance(10); advance(29000); }, { answers: [{ status: 429, retryAfter: '30' }, 202] });
  assert.equal(fetches(r).length, 1);
  const r2 = run(PLAIN, ({ advance }) => { advance(10); advance(31000); }, { answers: [{ status: 429, retryAfter: '30' }, 202] });
  assert.equal(fetches(r2).length, 2);
  // A Retry-After beyond five minutes counts as five minutes.
  const r3 = run(PLAIN, ({ advance }) => { advance(10); advance(299000); }, { answers: [{ status: 503, retryAfter: '86400' }, 202] });
  assert.equal(fetches(r3).length, 1);
  const r4 = run(PLAIN, ({ advance }) => { advance(10); advance(301000); }, { answers: [{ status: 503, retryAfter: '86400' }, 202] });
  assert.equal(fetches(r4).length, 2);
});

// verifies: REQ-TRACKER-2P41.S3 (400, 404 and 413 are final: the batch is dropped)
test('answers a retry cannot change drop the batch', () => {
  for (const status of [400, 404, 413]) {
    const r = run(PLAIN, ({ advance }) => { advance(10); advance(600000); }, { answers: [status] });
    assert.equal(fetches(r).length, 1, String(status));
  }
});

// verifies: REQ-TRACKER-2P41.S5 (one request in flight; later batches wait behind it and go in order after a success)
test('batches wait behind a failed one and are sent in order', () => {
  const r = run(PLAIN, ({ advance, lt }) => {
    advance(10);
    lt.track('one'); advance(3100);
    lt.track('two'); advance(3100);
    advance(10000);
  }, { answers: ['error', 202, 202, 202] });
  const c = fetches(r);
  const order = c.map((x) => decode(x.body).events.map((e) => e.name ?? e.type));
  assert.deepEqual(order, [[0], [0], ['one'], ['two']]);
  // Nothing new goes out while the first batch waits for its retry.
  assert.deepEqual(c.map((x) => x.at - c[0]!.at).slice(0, 2), [0, 1000]);
});

// verifies: REQ-TRACKER-2P41.S6 (an eleventh batch while ten wait drops the oldest)
test('at most ten batches wait', () => {
  const r = run({ ...PLAIN, 'data-batch': 'false' }, ({ advance, lt }) => {
    advance(10);
    for (let i = 1; i <= 12; i++) { lt.track('e' + i); advance(1); }
    advance(1200000);
  }, { answers: ['error', ...Array(20).fill(202)] });
  const names = fetches(r).map((x) => decode(x.body).events[0]!.name ?? 'pv');
  // The first batch (in flight) stays; of the rest the two oldest were dropped.
  assert.equal(names[0], 'pv');
  assert.ok(!names.includes('e1') && !names.includes('e2'), names.join(','));
  assert.deepEqual(names.slice(-10).slice(-3), ['e10', 'e11', 'e12']);
});

// verifies: REQ-TRACKER-2P41.S7 (a batch that has waited more than an hour is dropped)
test('a batch older than an hour is dropped', () => {
  const r = run(PLAIN, ({ advance }) => { advance(10); advance(3 * 3600000); }, { answers: Array(200).fill('error') });
  const c = fetches(r);
  assert.ok(c.every((x) => ageOf(x) <= 3600000));
  assert.ok(c.length < 200);
  const last = c[c.length - 1]!;
  const stopped = last.at;
  assert.ok(stopped - c[0]!.at <= 3600000 + 60000);
});

// verifies: REQ-TRACKER-2P41.S9 (retries write nothing to device storage)
test('retrying never writes to storage', () => {
  const r = run(PLAIN, ({ advance }) => { advance(10); advance(600000); }, { answers: ['error', 'error', 'error'] });
  assert.ok(fetches(r).length >= 3);
  assert.deepEqual(r.storage.filter((x) => x.startsWith('set:')), []);
});

// verifies: REQ-TRACKER-2P41.S10 (opted out before a retry: the queue is emptied and nothing is sent)
test('opting out empties the retry queue', () => {
  let out = false;
  const r = run(PLAIN, ({ advance }) => { advance(10); out = true; advance(600000); }, { answers: ['error', 202], optOut: () => out });
  assert.equal(fetches(r).length, 1);
});
