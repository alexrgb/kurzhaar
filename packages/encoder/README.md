# @kurzhaar/encoder

Pure TypeScript encoder for wire format v1 (ADR-T7N1), meant for the Node SDK and other server-side TypeScript code.

```ts
import { encodeBatch } from '@kurzhaar/encoder';

const body = encodeBatch({ key: 'abc', lang: 'en' }, [
  { ts: 1731150000000, url: '/' },
  { ts: 1731150004000, type: 1, name: 'signup', props: { plan: 'pro', trial: true } },
]);
// v::1|k::abc|l::en
// ts::1731150000000|u::/
// d::4000|y::1|n::signup|p.plan::pro|p.trial::1
```

The decoder is `@kurzhaar/protocol`. The browser tracker keeps its own inline encoder and does not use this package.
