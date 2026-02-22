# Task: Module Forms + Attributes

> **Status:** Done
> `HEARTBEAT_TASK_STATUS=done`


## Goal
Add module-level compile-time forms required for practical code organization.

## Scope
- `alias`, `import`, `require`, `use` (minimal semantics first).
- Module attributes (`@moduledoc`, `@doc`, and generic `@attr` storage).
- Resolver integration for imported/aliased names.
- Diagnostics for unsupported/invalid forms.

## Out of Scope
- Full protocol behavior implementation.
- Complete macro system parity.

## Deliverables
- Parser and resolver support for module forms.
- Attribute storage and serialization where needed for tooling/docs.

## Acceptance Criteria
- Aliased/imported function references resolve correctly in supported cases.
- Attributes parse cleanly and are retrievable by tooling hooks.

## Verification
- `cargo test`
- Add parser/resolver tests and CLI smoke coverage where needed.

## Suggested Commit
`feat(parity): add module forms and basic attribute support`
