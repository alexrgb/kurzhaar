---
id: ADR-03W7
type: adr
status: accepted
---
# ADR-03W7: Write all JavaScript-side code in TypeScript

## Context

The repository started with a plain JavaScript prototype of the tracker and decoder. The tracker has since moved to TypeScript. Four tracked files are still JavaScript: the reference decoder `packages/protocol/src/decode.js`, its test, the tracker request test and the tracker build script. Mixed languages mean two lint and typing regimes, untyped wire-format code and no shared types between decoder, encoder and tracker.

## Decision

All code that runs on Node or in the browser is TypeScript: libraries, tests, build scripts, applications (NestJS, Next.js) and SDK sources. Plain `.js`, `.mjs`, `.cjs` and `.jsx` source files are not committed. The only JavaScript in the repository is generated output (for example `dist/lt.js`, the built tracker), which is not tracked.

- Node 24 runs TypeScript directly, so tests and scripts need no build step. Code must be erasable syntax only (`erasableSyntaxOnly`): no enums, namespaces or constructor parameter properties.
- Each package has a `tsconfig.json` with `strict` and a `typecheck` script; CI runs `pnpm typecheck`.
- Packages that other packages import (`@kurzhaar/protocol`) ship types.
- Config files that a tool requires in JavaScript are allowed only when the tool offers no TypeScript form, and are listed here when added.

## Consequences

- One language and one type system across tracker, protocol, encoder, schema tools and apps.
- The wire-format decoder gets types shared with the encoder.
- A CI check fails on a committed `.js`, `.mjs`, `.cjs` or `.jsx` source file.
- Tests that import packages must resolve `.ts` entry points, so package `main` and `exports` point at TypeScript sources inside the monorepo.
