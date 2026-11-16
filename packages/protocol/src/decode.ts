// Reference decoder for wire format v1, revision 1.2 (see contracts/protocol/fields.json).

export interface DecodedEvent {
  ts: number;
  type: number;
  name: string | undefined;
  url: string | undefined;
  title: string | undefined;
  referrer: string | undefined;
  props: Record<string, string>;
  /** `g` tags (revision 1.2); absent when the event has none. */
  tags?: string[];
  /** `y::3` engagement: engaged time in ms (`et`) and maximum scroll percent (`sc`). */
  engagedMs?: number;
  scrollPct?: number;
  /** `y::4` performance: Web Vitals (`lcp`, `inp`, `fcp`, `ttfb` in ms, `cls` unitless). */
  lcp?: number;
  inp?: number;
  cls?: number;
  fcp?: number;
  ttfb?: number;
}

export interface DecodedBatch {
  key: string | undefined;
  screen: string | undefined;
  lang: string | undefined;
  identity: string | undefined;
  /** `tv`: version of the tracker script (revision 1.2). */
  trackerVersion?: string;
  /** `tz`: visitor offset from UTC in minutes, negative west of UTC (revision 1.2). */
  tzOffset?: number;
  events: DecodedEvent[];
}

const unesc = (s: string): string =>
  s.replace(/%([0-9A-F]{2})/g, (_, h: string) => String.fromCharCode(parseInt(h, 16)));

/** Optional numeric field: absent stays undefined; not a number (or not whole when `whole`) is an error. */
function optNum(o: Record<string, string>, k: string, whole: boolean): number | undefined {
  const v = o[k];
  if (v === undefined) return undefined;
  const n = Number(v);
  if (v.trim() === '' || !Number.isFinite(n) || (whole && !Number.isInteger(n))) throw new Error('invalid number: ' + v);
  return n;
}

type ExtraKey = 'engagedMs' | 'scrollPct' | 'lcp' | 'inp' | 'cls' | 'fcp' | 'ttfb';
// wire key, decoded key, whole number only (everything except cls is whole)
const EXTRA: [string, ExtraKey, boolean][] = [
  ['et', 'engagedMs', true], ['sc', 'scrollPct', true], ['lcp', 'lcp', true], ['inp', 'inp', true],
  ['cls', 'cls', false], ['fcp', 'fcp', true], ['ttfb', 'ttfb', true],
];

/** Splits a line into fields. Values stay escaped, so a list field can be split before it is unescaped. */
function parseLine(line: string): Record<string, string> {
  const o: Record<string, string> = {};
  for (const part of line.split('|')) {
    const i = part.indexOf('::');
    if (i > 0) o[part.slice(0, i)] = part.slice(i + 2);
  }
  return o;
}

/** Unescaped value of a plain field. */
const str = (o: Record<string, string>, k: string): string | undefined => (o[k] === undefined ? undefined : unesc(o[k]));

/** `g` tags: split on commas first, then unescape each tag, so `%2C` stays inside a tag. */
function tagsOf(o: Record<string, string>): string[] | undefined {
  const raw = o.g;
  if (raw === undefined || raw === '') return undefined;
  const tags = raw.split(',').filter((t) => t !== '').map(unesc);
  return tags.length ? tags : undefined;
}

export function decode(body: string): DecodedBatch {
  const [head = '', ...rest] = body.split('\n');
  const h = parseLine(head);
  if (h.v !== '1') throw new Error('unsupported protocol version: ' + (h.v === undefined ? h.v : unesc(h.v)));
  let t = 0;
  const events = rest.map((line): DecodedEvent => {
    const o = parseLine(line);
    t = o.ts !== undefined ? Number(o.ts) : t + Number(o.d || 0);
    const props: Record<string, string> = {};
    for (const k in o) if (k.startsWith('p.')) props[unesc(k.slice(2))] = unesc(o[k] as string);
    const e: DecodedEvent = { ts: t, type: Number(o.y || 0), name: str(o, 'n'), url: str(o, 'u'), title: str(o, 't'), referrer: str(o, 'r'), props };
    const tags = tagsOf(o);
    if (tags) e.tags = tags;
    for (const [field, key, whole] of EXTRA) {
      const n = optNum(o, field, whole);
      if (n !== undefined) e[key] = n;
    }
    return e;
  });
  const batch: DecodedBatch = { key: str(h, 'k'), screen: str(h, 's'), lang: str(h, 'l'), identity: str(h, 'i'), events };
  const tv = str(h, 'tv');
  if (tv !== undefined) batch.trackerVersion = tv;
  const tz = optNum(h, 'tz', true);
  if (tz !== undefined) batch.tzOffset = tz;
  return batch;
}
