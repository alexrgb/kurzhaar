// Reference decoder for wire format v1 (see encode() in tracker.js).
const unesc = s => s.replace(/%([0-9A-F]{2})/g, (_, h) => String.fromCharCode(parseInt(h, 16)));

function parseLine(line) {
  const o = {};
  for (const part of line.split('|')) {
    const i = part.indexOf('::');
    if (i > 0) o[part.slice(0, i)] = unesc(part.slice(i + 2));
  }
  return o;
}

function decode(body) {
  const [head, ...rest] = body.split('\n');
  const h = parseLine(head);
  if (h.v !== '1') throw new Error('unsupported protocol version: ' + h.v);
  let t = 0;
  const events = rest.map(line => {
    const o = parseLine(line);
    t = o.ts !== undefined ? Number(o.ts) : t + Number(o.d || 0);
    const e = { ts: t, type: Number(o.y || 0), name: o.n, url: o.u, title: o.t, referrer: o.r, props: {} };
    for (const k in o) if (k.startsWith('p.')) e.props[unesc(k.slice(2))] = o[k];
    return e;
  });
  return { key: h.k, screen: h.s, lang: h.l, identity: h.i, events };
}

module.exports = { decode };
