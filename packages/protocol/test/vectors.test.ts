// verifies: conformance of the reference decoder with contracts/protocol/vectors.json
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { decode } from '../src/decode.ts';

interface Vector {
  name: string;
  body: string;
  expected?: unknown;
  error?: string;
}

const { vectors } = JSON.parse(
  readFileSync(new URL('../../../contracts/protocol/vectors.json', import.meta.url), 'utf8'),
) as { vectors: Vector[] };

for (const v of vectors) {
  test(v.name, () => {
    if (v.error) {
      assert.throws(() => decode(v.body), (e: Error) => e.message.includes(v.error as string));
      return;
    }
    // JSON round trip drops undefined fields, matching the vector files.
    assert.deepEqual(JSON.parse(JSON.stringify(decode(v.body))), v.expected);
  });
}
