// verifies: conformance of the reference decoder with spec/protocol/vectors.json
const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const { decode } = require('../src/decode.js');
const { vectors } = require(path.join(__dirname, '../../../spec/protocol/vectors.json'));

for (const v of vectors) {
  test(v.name, () => {
    if (v.error) {
      assert.throws(() => decode(v.body), (e) => e.message.includes(v.error));
      return;
    }
    // JSON round trip drops undefined fields, matching the vector files.
    assert.deepEqual(JSON.parse(JSON.stringify(decode(v.body))), v.expected);
  });
}
