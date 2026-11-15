/** Minimal ClickHouse HTTP client (no dependencies). */
export interface Options {
  url: string;
  user?: string;
  password?: string;
}

export function fromEnv(): Options {
  return {
    url: process.env.CLICKHOUSE_URL ?? 'http://127.0.0.1:8123',
    user: process.env.CLICKHOUSE_USER,
    password: process.env.CLICKHOUSE_PASSWORD,
  };
}

export class ClickHouse {
  private readonly opts: Options;

  constructor(opts: Options) {
    this.opts = opts;
  }

  private async post(sqlInUrl: boolean, sql: string, extra: Record<string, string>, body?: string): Promise<string> {
    const u = new URL(this.opts.url);
    for (const [k, v] of Object.entries(extra)) u.searchParams.set(k, v);
    if (sqlInUrl) u.searchParams.set('query', sql);
    const headers: Record<string, string> = {};
    if (this.opts.user) headers['X-ClickHouse-User'] = this.opts.user;
    if (this.opts.password) headers['X-ClickHouse-Key'] = this.opts.password;
    const res = await fetch(u, { method: 'POST', headers, body: sqlInUrl ? body : sql });
    const text = await res.text();
    if (!res.ok) throw new Error(`ClickHouse ${res.status}: ${text.trim()}`);
    return text;
  }

  /** Run a statement that returns nothing. */
  exec(sql: string, settings: Record<string, string> = {}): Promise<string> {
    return this.post(false, sql, settings);
  }

  /** Run a SELECT with query parameters ({name:Type}); rows come back as objects. */
  async query<T = Record<string, unknown>>(
    sql: string,
    params: Record<string, string | number> = {},
    settings: Record<string, string> = {},
  ): Promise<T[]> {
    const extra: Record<string, string> = { ...settings, default_format: 'JSONEachRow', output_format_json_quote_64bit_integers: '0' };
    for (const [k, v] of Object.entries(params)) extra[`param_${k}`] = String(v);
    const text = await this.post(false, sql, extra);
    return text
      .split('\n')
      .filter(Boolean)
      .map((l) => JSON.parse(l) as T);
  }

  /** Insert JSON rows. Pass `token` to make a retry of the same batch a no-op. */
  async insert(table: string, rows: object[], token?: string): Promise<void> {
    const settings: Record<string, string> = {
      date_time_input_format: 'best_effort',
      deduplicate_blocks_in_dependent_materialized_views: '1',
    };
    if (token) settings.insert_deduplication_token = token;
    await this.post(true, `INSERT INTO ${table} FORMAT JSONEachRow`, settings, rows.map((r) => JSON.stringify(r)).join('\n'));
  }
}
