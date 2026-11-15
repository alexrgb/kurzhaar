---
id: REQ-TRACKER-2D61
type: requirement
status: draft
links:
  decided-by:
    - ADR-03W7
---
### Requirement: All repository code that runs on Node or in the browser is TypeScript
The system SHALL keep every source file that runs on Node or in the browser in TypeScript, with strict type checking in CI.

#### Scenario: No JavaScript source is committed {#S1}
- **WHEN** CI runs on a change that adds a `.js`, `.mjs`, `.cjs` or `.jsx` source file
- **THEN** the check fails and names the file

#### Scenario: Types are checked {#S2}
- **WHEN** CI runs `pnpm typecheck`
- **THEN** every package compiles with `strict` and no type errors
