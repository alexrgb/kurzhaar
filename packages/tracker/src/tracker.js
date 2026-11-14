// Light analytics tracker. No cookies, no deps, optional batching, SPA-safe.
// Config (script attributes): data-key (required), data-host, data-query ("all"|"none"; default: utm/click ids only),
// data-auto ("false" disables automatic page views), data-dnt ("true" honours Do Not Track),
// data-batch ("false" sends every event immediately; default on: the first event goes out at once,
// the rest are buffered), data-batch-ms (buffer window after the first event, default 3000)
(function (w, d) {
  var s = d.currentScript;
  if (!s) return;
  var key = s.getAttribute('data-key');
  if (!key) return;
  var host = (s.getAttribute('data-host') || s.src.replace(/\/[^/]*$/, '')).replace(/\/$/, '');
  var url = host + '/v1/collect';
  var queryMode = s.getAttribute('data-query');
  var h = w.history, loc = w.location, nav = w.navigator;
  var batch = s.getAttribute('data-batch') !== 'false';
  var win = +s.getAttribute('data-batch-ms') || 3000;
  var q = [], timer, ident, lastPath = null, sent = 0;

  // Query params kept by default; everything else is stripped client-side (may contain tokens or emails).
  var KEEP = /^(utm_\w+|gclid|fbclid|msclkid|ttclid|twclid|li_fat_id|ref)$/;

  function off() {
    try {
      if (w.localStorage.getItem('lt.off')) return 1;
    } catch (e) {}
    return s.getAttribute('data-dnt') === 'true' && nav.doNotTrack === '1';
  }

  function clean(u) {
    try {
      u = new URL(u, loc.href);
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
  //   v::1|k::<key>|s::<WxH>|l::<lang>|i::<id>          <- line 1: batch header
  //   ts::<epoch ms>|u::/|t::Title|r::google.com/        <- first event: full timestamp (pageview, y omitted = 0)
  //   d::4000|y::1|n::signup|u::/pricing|p.plan::pro     <- next events: delta ms since the previous event
  // y: 0 pageview (default, omitted), 1 custom event, 2 identify. Booleans are sent as 1/0.
  // Escaping (values and prop names): % | \n \r -> %XX; prop names also escape ":".
  function esc(v) {
    return String(v).replace(/[%|\n\r]/g, function (c) {
      return '%' + ('0' + c.charCodeAt(0).toString(16)).slice(-2).toUpperCase();
    });
  }

  function val(v) {
    return v === true ? 1 : v === false ? 0 : typeof v === 'object' ? JSON.stringify(v) : v;
  }

  function pair(k, v) {
    return v == null || v === '' ? '' : k + '::' + esc(val(v));
  }

  function encode(events) {
    var head = ['v::1', pair('k', key), pair('s', d.documentElement.clientWidth + 'x' + d.documentElement.clientHeight), pair('l', nav.language), pair('i', ident)];
    var lines = [head.filter(Boolean).join('|')];
    var prev = 0;
    events.forEach(function (e, n) {
      var f = [n ? pair('d', e.ts - prev) : pair('ts', e.ts), e.y ? pair('y', e.y) : '', pair('n', e.n), pair('u', e.u), pair('t', e.t), pair('r', e.r)];
      prev = e.ts;
      for (var k in e.p) f.push(pair('p.' + esc(k).replace(/:/g, '%3A'), e.p[k]));
      lines.push(f.filter(Boolean).join('|'));
    });
    return lines.join('\n');
  }

  function flush() {
    clearTimeout(timer);
    timer = 0;
    if (!q.length || off()) return (q = []);
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

  function push(ev) {
    ev.ts = Date.now();
    q.push(ev);
    // Leading edge: the first event of a page load is sent at once (it is the most valuable one),
    // later events wait up to `win` ms, or until 20 are queued, or the page is hidden.
    if (!batch || !sent || q.length >= 20) flush();
    else if (!timer) timer = setTimeout(flush, win);
  }

  function view(ref) {
    var u = clean(loc.href);
    if (!u) return;
    var path = u.pathname;
    // Only a pathname change is a new page; query-only changes (filters, tabs) are not.
    if (path === lastPath) return;
    var prev = lastPath;
    lastPath = path;
    push({ y: 0, u: path + u.search, t: d.title, r: prev === null ? ref : prev });
  }

  function track(name, props) {
    push({ y: 1, n: String(name).slice(0, 64), u: loc.pathname, p: props });
  }

  function identify(id, traits) {
    ident = String(id).slice(0, 64);
    push({ y: 2, p: traits });
  }

  function refOf(u) {
    try {
      var r = new URL(u);
      return r.origin === loc.origin ? r.pathname : r.origin + r.pathname;
    } catch (e) {
      return '';
    }
  }

  function hook(m) {
    var orig = h[m];
    h[m] = function () {
      var out = orig.apply(this, arguments);
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

  // <button data-lt-event="signup" data-lt-plan="pro">: no preventDefault, the beacon survives navigation.
  d.addEventListener(
    'click',
    function (e) {
      var el = e.target && e.target.closest && e.target.closest('[data-lt-event]');
      if (!el) return;
      var p = {};
      for (var i = 0, a = el.attributes; i < a.length; i++) {
        if (a[i].name.indexOf('data-lt-') === 0 && a[i].name !== 'data-lt-event') p[a[i].name.slice(8)] = a[i].value;
      }
      track(el.getAttribute('data-lt-event'), p);
    },
    true
  );

  d.addEventListener('visibilitychange', function () {
    if (d.visibilityState === 'hidden') flush();
  });
  w.addEventListener('pagehide', flush);
})(window, document);
