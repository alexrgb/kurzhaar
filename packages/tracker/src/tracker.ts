// Light analytics tracker. No cookies, no deps, optional batching, SPA-safe.
// Config (script attributes): data-key (required), data-host, data-query ("all"|"none"; default: utm/click ids only),
// data-auto ("false" disables automatic page views), data-dnt ("true" honours Do Not Track),
// data-batch ("false" sends every event immediately; default on: the first event goes out at once,
// the rest are buffered), data-batch-ms (buffer window after the first event, default 3000)
type Props = Record<string, unknown>;
// Tracker version, replaced by esbuild at build time from package.json (wire key `tv`).
declare const __VERSION__: string;
// User-Agent Client Hints (Chromium only); not in the DOM typings.
interface Uad {
  brands?: { brand: string; version: string }[];
  mobile?: boolean;
  platform?: string;
  getHighEntropyValues?(hints: string[]): { then(ok: (v?: { platformVersion?: string }) => void, err: () => void): unknown };
}
interface Ev {
  y: number;
  ts: number;
  n?: string;
  u?: string;
  t?: string;
  r?: string;
  g?: string[];
  p?: Props;
}
interface Lt {
  track(name: string, props?: Props, tags?: string[]): void;
  identify(id: string, traits?: Props): void;
  flush(): void;
}
(function (w: Window & { lt?: Lt }, d: Document) {
  var cur = d.currentScript as HTMLScriptElement | null;
  if (!cur) return;
  var s: HTMLScriptElement = cur;
  var key = s.getAttribute('data-key');
  if (!key) return;
  var host = (s.getAttribute('data-host') || s.src.replace(/\/[^/]*$/, '')).replace(/\/$/, '');
  var url = host + '/v1/collect';
  var queryMode = s.getAttribute('data-query');
  var h = w.history, loc = w.location, nav = w.navigator;
  var batch = s.getAttribute('data-batch') !== 'false';
  var win = Number(s.getAttribute('data-batch-ms')) || 3000;
  var q: Ev[] = [],
    timer: ReturnType<typeof setTimeout> | 0 = 0,
    ident: string | undefined,
    lastPath: string | null = null,
    sent = 0,
    // Device signals for server-side detection (ADR-ZKD1). `cv` (platform major version) is async.
    uad = (nav as Navigator & { userAgentData?: Uad }).userAgentData,
    cv: string | undefined,
    waiting = false,
    hintTimer: ReturnType<typeof setTimeout> | 0 = 0;

  // Query params kept by default; everything else is stripped client-side (may contain tokens or emails).
  var KEEP = /^(utm_\w+|gclid|fbclid|msclkid|ttclid|twclid|li_fat_id|gad_source|dclid|wbraid|gbraid|mc_cid|igshid|ref)$/;

  function off(): boolean {
    try {
      if (w.localStorage.getItem('lt.off')) return true;
    } catch (e) {}
    return s.getAttribute('data-dnt') === 'true' && nav.doNotTrack === '1';
  }

  function clean(href: string): URL | null {
    try {
      var u = new URL(href, loc.href);
      if (queryMode !== 'all') {
        var p = new URLSearchParams();
        if (queryMode !== 'none')
          u.searchParams.forEach(function (v, k) {
            if (KEEP.test(k)) p.append(k, v);
          });
        u.search = p.toString();
      }
      u.hash = '';
      return u;
    } catch (e) {
      return null;
    }
  }

  // Wire format v1: one event per line, fields as key::value joined by "|".
  //   v::1|k::<key>|s::<WxH>|l::<lang>|i::<id>|tv::<version>|tz::<minutes>   <- line 1: batch header
  //   ts::<epoch ms>|u::/|t::Title|r::google.com/        <- first event: full timestamp (pageview, y omitted = 0)
  //   d::4000|y::1|n::signup|u::/pricing|g::paid,trial|p.plan::pro   <- next events: delta ms since the previous event
  // y: 0 pageview (default, omitted), 1 custom event, 2 identify. Booleans are sent as 1/0.
  // Escaping (values and prop names): % | \n \r -> %XX; prop names also escape ":".
  function esc(v: unknown): string {
    return String(v).replace(/[%|\n\r]/g, function (c) {
      return '%' + ('0' + c.charCodeAt(0).toString(16)).slice(-2).toUpperCase();
    });
  }

  function val(v: unknown): unknown {
    return v === true ? 1 : v === false ? 0 : typeof v === 'object' ? JSON.stringify(v) : v;
  }

  function pair(k: string, v: unknown): string {
    return v == null || v === '' ? '' : k + '::' + esc(val(v));
  }

  // Each item is escaped on its own and "," inside an item becomes %2C; the joined value is not escaped again.
  function listPair(k: string, g?: string[]): string {
    return g && g.length
      ? k + '::' +
          g
            .map(function (t) {
              return esc(t).replace(/,/g, '%2C');
            })
            .join(',')
      : '';
  }

  // At most 10 tags of 32 characters; empty tags are dropped.
  function tagList(tags?: unknown): string[] | undefined {
    if (!Array.isArray(tags)) return undefined;
    var out: string[] = [];
    for (var i = 0; i < tags.length && out.length < 10; i++) {
      var t = String(tags[i]).trim().slice(0, 32);
      if (t) out.push(t);
    }
    return out.length ? out : undefined;
  }

  // Brands, mobile flag, platform, platform major version, touch points and pointer type. Never a model or build number.
  function signals(): string[] {
    var coarse = w.matchMedia ? (w.matchMedia('(pointer:coarse)').matches ? 1 : 0) : undefined;
    var f = [pair('tp', nav.maxTouchPoints), pair('pc', coarse)];
    if (uad)
      f.unshift(
        listPair('cb', (uad.brands || []).map(function (b) { return b.brand + '/' + b.version; })),
        pair('cm', uad.mobile ? 1 : 0),
        pair('cp', uad.platform),
        pair('cv', cv)
      );
    return f;
  }

  function encode(events: Ev[]): string {
    var head = ['v::1', pair('k', key), pair('s', d.documentElement.clientWidth + 'x' + d.documentElement.clientHeight), pair('l', nav.language), pair('i', ident), pair('tv', __VERSION__), pair('tz', 0 - new Date().getTimezoneOffset())].concat(signals());
    var lines = [head.filter(Boolean).join('|')];
    var prev = 0;
    events.forEach(function (e, n) {
      var f = [n ? pair('d', e.ts - prev) : pair('ts', e.ts), e.y ? pair('y', e.y) : '', pair('n', e.n), pair('u', e.u), pair('t', e.t), pair('r', e.r), listPair('g', e.g)];
      prev = e.ts;
      for (var k in e.p) f.push(pair('p.' + esc(k).replace(/:/g, '%3A'), e.p[k]));
      lines.push(f.filter(Boolean).join('|'));
    });
    return lines.join('\n');
  }

  function flush(): void {
    clearTimeout(timer || undefined);
    timer = 0;
    if (!q.length || off()) {
      q = [];
      return;
    }
    var body = encode(q);
    q = [];
    sent = 1;
    // text/plain avoids a CORS preflight; beacons survive page unload.
    if (!(nav.sendBeacon && nav.sendBeacon(url, body))) {
      try {
        fetch(url, { method: 'POST', body: body, keepalive: true, credentials: 'omit' });
      } catch (e) {}
    }
  }

  function push(ev: Omit<Ev, 'ts'>): void {
    (ev as Ev).ts = Date.now();
    q.push(ev as Ev);
    // Leading edge: the first event of a page load is sent at once (it is the most valuable one),
    // later events wait up to `win` ms, or until 20 are queued, or the page is hidden.
    if (!batch || !sent || q.length >= 20) {
      // The first send of a page load waits up to 150 ms for the platform version (REQ-TRACKER-EZ0J).
      if (waiting) hintTimer = hintTimer || setTimeout(release, 150);
      else flush();
    } else if (!timer) timer = setTimeout(flush, win);
  }

  function release(): void {
    clearTimeout(hintTimer || undefined);
    hintTimer = 0;
    waiting = false;
    flush();
  }

  if (uad && uad.getHighEntropyValues) {
    waiting = true;
    try {
      uad.getHighEntropyValues(['platformVersion']).then(
        function (v) {
          var m = v && v.platformVersion ? parseInt(v.platformVersion, 10) : NaN;
          if (m === m) cv = String(m);
          release();
        },
        release
      );
    } catch (e) {
      waiting = false;
    }
  }

  function view(ref?: string): void {
    var u = clean(loc.href);
    if (!u) return;
    var path = u.pathname;
    // Only a pathname change is a new page; query-only changes (filters, tabs) are not.
    if (path === lastPath) return;
    var prev = lastPath;
    lastPath = path;
    push({ y: 0, u: path + u.search, t: d.title, r: prev === null ? ref : prev });
  }

  function track(name: string, props?: Props, tags?: string[]): void {
    push({ y: 1, n: String(name).slice(0, 64), u: loc.pathname, p: props, g: tagList(tags) });
  }

  function identify(id: string, traits?: Props): void {
    ident = String(id).slice(0, 64);
    push({ y: 2, p: traits });
  }

  function refOf(u: string): string {
    try {
      var r = new URL(u);
      return r.origin === loc.origin ? r.pathname : r.origin + r.pathname;
    } catch (e) {
      return '';
    }
  }

  function hook(m: 'pushState' | 'replaceState'): void {
    var orig = h[m];
    h[m] = function (this: History) {
      var out = orig.apply(this, arguments as unknown as Parameters<typeof orig>);
      // Defer so the router can update document.title before we read it.
      setTimeout(view, 0);
      return out;
    };
  }

  w.lt = w.lt || { track: track, identify: identify, flush: flush };

  if (s.getAttribute('data-auto') !== 'false') {
    view(d.referrer ? refOf(d.referrer) : '');
    hook('pushState');
    hook('replaceState');
    w.addEventListener('popstate', function () {
      view();
    });
  }

  // <button data-lt-event="signup" data-lt-tags="paid,trial" data-lt-plan="pro">: no preventDefault, the beacon survives navigation.
  d.addEventListener(
    'click',
    function (e) {
      var t = e.target as Element | null;
      var el = t && t.closest && t.closest('[data-lt-event]');
      if (!el) return;
      var p: Props = {};
      for (var i = 0, a = el.attributes; i < a.length; i++) {
        if (a[i].name.indexOf('data-lt-') === 0 && a[i].name !== 'data-lt-event' && a[i].name !== 'data-lt-tags') p[a[i].name.slice(8)] = a[i].value;
      }
      var tg = el.getAttribute('data-lt-tags');
      track(el.getAttribute('data-lt-event') as string, p, tg ? tg.split(',') : undefined);
    },
    true
  );

  d.addEventListener('visibilitychange', function () {
    if (d.visibilityState === 'hidden') flush();
  });
  w.addEventListener('pagehide', flush);
})(window, document);
