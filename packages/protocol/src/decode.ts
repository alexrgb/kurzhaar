// Reference decoder for wire format v1 (see encode() in packages/tracker/src/tracker.ts).

export interface DecodedEvent {
  ts: number;
  type: number;
  name: string | undefined;
  url: string | undefined;
  title: string | undefined;
  referrer: string | undefined;
  props: Record<string, string>;
}

export interface DecodedBatch {
  key: string | undefined;
  screen: string | undefined;
  lang: string | undefined;
  identity: string | undefined;
  events: DecodedEvent[];
}

const unesc = (s: string): string =>
  s.replace(/%([0-9A-F]{2})/g, (_, h: string) => String.fromCharCode(parseInt(h, 16)));

function parseLine(line: string): Record<string, string> {
  const o: Record<string, string> = {};
  for (const part of line.split('|')) {
    const i = part.indexOf('::');
    if (i > 0) o[part.slice(0, i)] = unesc(part.slice(i + 2));
  }
  return o;
}

export function decode(body: string): DecodedBatch {
  const [head = '', ...rest] = body.split('\n');
  const h = parseLine(head);
  if (h.v !== '1') throw new Error('unsupported protocol version: ' + h.v);
  let t = 0;
  const events = rest.map((line): DecodedEvent => {
    const o = parseLine(line);
    t = o.ts !== undefined ? Number(o.ts) : t + Number(o.d || 0);
    const props: Record<string, string> = {};
    for (const k in o) if (k.startsWith('p.')) props[unesc(k.slice(2))] = o[k] as string;
    return { ts: t, type: Number(o.y || 0), name: o.n, url: o.u, title: o.t, referrer: o.r, props };
  });
  return { key: h.k, screen: h.s, lang: h.l, identity: h.i, events };
}
