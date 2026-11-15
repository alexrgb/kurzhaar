// Encoder for wire format v1 (ADR-T7N1). Pure functions, no I/O, no dependencies.
//
//   v::1|k::<key>|s::<WxH>|l::<lang>|i::<id>      line 1: batch header
//   ts::<epoch ms>|u::/|t::Title|r::ref            first event: full timestamp
//   d::4000|y::1|n::signup|p.plan::pro             later events: delta ms from the previous event
//   d::8000|y::3|u::/|et::7500|sc::80              engagement; y::4 carries lcp/inp/cls/fcp/ttfb (ADR-B0B4)
//
// The decoder reference lives in @kurzhaar/protocol; the browser tracker keeps its own
// inline encoder to stay tiny and is intentionally not built on this package.

/** Event type: 0 pageview (omitted on the wire), 1 custom event, 2 identify, 3 engagement, 4 performance. */
export type EventType = 0 | 1 | 2 | 3 | 4;

/** A property value. Booleans become 1/0, objects JSON, null/undefined/"" are dropped. */
export type PropValue = string | number | boolean | null | undefined | object;

export interface BatchHeader {
  /** Project key (`k`). */
  key?: string;
  /** Screen size such as "1920x1080" (`s`). */
  screen?: string;
  /** Language tag such as "uk-UA" (`l`). */
  lang?: string;
  /** Identified user id (`i`). */
  identity?: string;
}

export interface WireEvent {
  /** Absolute time, integer epoch milliseconds. */
  ts: number;
  /** Defaults to 0 (pageview). */
  type?: EventType;
  name?: string;
  url?: string;
  title?: string;
  referrer?: string;
  /** Type 3: engaged time in ms (`et`) and maximum scroll percent 0-100 (`sc`). */
  engagedMs?: number;
  scrollPct?: number;
  /** Type 4: Web Vitals (`lcp`, `inp`, `fcp`, `ttfb` in ms, `cls` unitless). Absent values are omitted. */
  lcp?: number;
  inp?: number;
  cls?: number;
  fcp?: number;
  ttfb?: number;
  props?: Record<string, PropValue>;
}

/** Escapes `%`, `|`, CR and LF as `%XX` (uppercase hex). */
export function escapeValue(v: string): string {
  return v.replace(/[%|\n\r]/g, (c) => '%' + c.charCodeAt(0).toString(16).toUpperCase().padStart(2, '0'));
}

/** Like {@link escapeValue}, and also escapes `:` so a name can never contain `::`. */
export function escapePropName(v: string): string {
  return escapeValue(v).replace(/:/g, '%3A');
}

function stringify(v: PropValue): string {
  if (v === true) return '1';
  if (v === false) return '0';
  if (v === null || v === undefined) return '';
  if (typeof v === 'object') return JSON.stringify(v) ?? '';
  return String(v);
}

function field(key: string, v: PropValue): string | undefined {
  const s = stringify(v);
  return s === '' ? undefined : key + '::' + escapeValue(s);
}

function join(fields: (string | undefined)[]): string {
  return fields.filter((f): f is string => f !== undefined).join('|');
}

/** Encodes the header line. The version is always the first field. */
export function encodeHeader(header: BatchHeader = {}): string {
  return join(['v::1', field('k', header.key), field('s', header.screen), field('l', header.lang), field('i', header.identity)]);
}

function checkTs(ts: number): void {
  if (!Number.isSafeInteger(ts)) throw new RangeError('event ts must be an integer number of epoch milliseconds: ' + ts);
}

/**
 * Encodes events, one line each. The first carries a full `ts`, the rest `d` deltas from
 * the previous event in the given order (a negative delta is legal and round-trips).
 * Returns an empty array for no events.
 */
export function encodeEvents(events: readonly WireEvent[]): string[] {
  let prev = 0;
  return events.map((e, i) => {
    checkTs(e.ts);
    const f: (string | undefined)[] = [
      i === 0 ? 'ts::' + e.ts : 'd::' + (e.ts - prev),
      e.type ? 'y::' + e.type : undefined,
      field('n', e.name),
      field('u', e.url),
      field('t', e.title),
      field('r', e.referrer),
      field('et', e.engagedMs),
      field('sc', e.scrollPct),
      field('lcp', e.lcp),
      field('inp', e.inp),
      field('cls', e.cls),
      field('fcp', e.fcp),
      field('ttfb', e.ttfb),
    ];
    prev = e.ts;
    for (const k of Object.keys(e.props ?? {})) {
      const s = field('p.' + escapePropName(k), e.props?.[k]);
      if (s !== undefined) f.push(s);
    }
    return join(f);
  });
}

/** Encodes a full batch body (header line, then one line per event, joined by LF). */
export function encodeBatch(header: BatchHeader, events: readonly WireEvent[]): string {
  return [encodeHeader(header), ...encodeEvents(events)].join('\n');
}
