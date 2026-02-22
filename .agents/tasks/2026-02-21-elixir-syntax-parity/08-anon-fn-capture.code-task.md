# Task: Anonymous Functions + Capture

> **Status:** Done
> `HEARTBEAT_TASK_STATUS=done`


## Goal
Support `fn ... end`, capture syntax (`&`, `&1`), and invocation (`fun.(arg)`).

## Scope
- Parser support for anonymous function literals and capture forms.
- Runtime function values / closures (minimum lexical capture model).
- Invocation syntax for function values.
- Basic capture expansion semantics.

## Out of Scope
- Full macro/metaprogramming function generation.

## Deliverables
- AST/IR/runtime representation for function values.
- Tests for creation, capture, and invocation semantics.

## Acceptance Criteria
- Anonymous functions execute with expected argument binding.
- Capture shorthand (`&(&1 + 1)`-style subset) works for supported forms.
- `fun.(x)` invocation routes correctly.

## Verification
- `cargo test`
- Add run/check smoke tests for anonymous-function pipelines.

## Suggested Commit
`feat(parity): add anonymous functions capture and function-value invocation`
