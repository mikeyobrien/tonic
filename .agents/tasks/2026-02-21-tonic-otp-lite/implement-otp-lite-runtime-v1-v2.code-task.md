# Task: Implement OTP-Lite Reliability Runtime for Tonic (v1/v2 MVP)

> **Status:** Superseded for current v0/v1 scope by `.agents/tasks/2026-02-21-tonic-reliability/harden-cli-runtime-reliability.code-task.md`.
> Keep this only as future/post-v1 exploration if product direction changes.

## Description
Add an OTP-inspired reliability layer to the Tonic runtime that delivers practical crash isolation and automated recovery semantics without attempting full BEAM parity. The MVP should provide actor-style process primitives, links/monitors, and a minimal one-for-one supervisor model with explicit overload/backpressure behavior.

The intent is to make Tonic robust enough for long-running CLI services and agent workloads while preserving a realistic delivery scope for a small team.

## Background
Research shows the highest-value OTP properties are semantic/runtime-layer guarantees (isolated processes, mailbox messaging, failure observation/propagation, supervision), not LLVM codegen itself. LLVM can accelerate execution, but reliability behavior comes from runtime orchestration and failure semantics.

Key findings from deep dive (`research/otp_lite_design_research.md`):
- Feasible in 3–6 months: isolated lightweight processes, async message passing, links/monitors, simplified supervision.
- Not feasible for MVP: BEAM-style preemptive reductions scheduler, per-process GC, hot code upgrades.
- Recommended stance: Tokio-based cooperative scheduling + injected yield points for CPU-heavy loops, bounded mailboxes for backpressure, and explicit restart-intensity limits.

This task should implement an OTP-lite subset aligned with those findings and integrate with current Tonic interpreter/runtime architecture.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- Research synthesis: `research/otp_lite_design_research.md`
- Runtime architecture context: `research/runtime-architecture.md`
- Implementation plan context: `.agents/planning/2026-02-20-elixir-tui-cli-language/implementation/plan.md`
- Current runtime implementation: `src/runtime.rs`
- Current IR/lowering model: `src/ir.rs`
- Current parser/front-end model: `src/parser.rs`
- Existing command/integration style: `tests/`

External references used for design constraints:
- Erlang/OTP Design Principles: https://www.erlang.org/doc/design_principles/des_princ.html
- ERTS architecture: https://www.erlang.org/doc/apps/erts/erts.html
- The BEAM Book (scheduler/reductions internals): https://github.com/happi/theBeamBook
- Learn You Some Erlang (errors/processes): https://learnyousomeerlang.com/errors-and-processes
- Learn You Some Erlang (supervision): https://learnyousomeerlang.com/designing-a-concurrent-application
- Tokio cooperative scheduling: https://tokio.rs/tokio/tutorial/spawning
- Erlang in Anger (overload/backpressure): https://www.erlang-in-anger.com/
- Handling overload notes: https://ferd.ca/handling-overload.html
- Firefly (Erlang on Rust/LLVM/Wasm): https://github.com/GetFirefly/firefly
- Lunatic (actor runtime in Rust/Wasm): https://lunatic.rs/

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Implement a runtime **Node** abstraction managing process lifecycle, PID allocation, and process registry.
2. Add a **Process** abstraction with isolated state, mailbox, lifecycle status (`running`, `exited`), and exit reason payload.
3. Introduce runtime primitives (internal API first) for:
   - `spawn`
   - `send`
   - `receive` (with timeout support)
   - `self`
4. Add **links** (bidirectional failure propagation): if a linked process exits abnormally, linked peers are notified/cancelled per configured semantics.
5. Add **monitors** (unidirectional observation): monitor owner receives deterministic DOWN-style notification when target exits.
6. Add a minimal **supervisor** abstraction with:
   - strategy: `one_for_one`
   - restart policy: `permanent` + `temporary` (at minimum)
   - restart intensity limit (e.g., max N restarts in T seconds)
7. Implement a minimal **gen_server-like behavior contract** (request/response loop) sufficient to model stateful services and timeout handling.
8. Enforce **bounded mailboxes** with configurable capacity and deterministic overflow behavior (`err(:mailbox_full)` or blocking variant; choose one and document).
9. Define explicit **failure taxonomy** and exit reasons (normal, shutdown, error/panic, timeout, mailbox overflow), with stable diagnostic rendering.
10. Add **cooperative fairness guardrails**:
    - define runtime yield points for long-running process loops
    - if LLVM execution path exists, define/implement insertion strategy for cooperative yield calls at loop/function boundaries
11. Keep **hot code upgrades** explicitly out of scope; document as a non-goal in runtime design notes and command help/docs where relevant.
12. Integrate these primitives incrementally without regressing existing interpreter semantics (`ok/err`, `?`, case, existing builtins).
13. Add machine-testable behavior contracts in integration tests for process messaging and supervisor recovery.
14. Ensure feature behavior is deterministic in single-node execution and deterministic enough for CI (bounded timing windows, no flaky sleeps).

## Dependencies
- Existing runtime evaluation pipeline (`src/runtime.rs`, `src/ir.rs`, `src/parser.rs`)
- Runtime state/data structures (`HashMap`, queues/channels)
- Async runtime choice (expected: `tokio`)
- Synchronization/channel primitives (e.g., `tokio::sync::mpsc`, `oneshot`, cancellation tokens)
- Diagnostic/reporting conventions from existing CLI/runtime error surface
- Existing integration test harness style in `tests/`

## Implementation Approach
1. **Scope guard + architecture doc (pre-code)**
   - Add/update runtime design note defining OTP-lite boundary:
     - in scope: spawn/mailbox/link/monitor/supervisor(one_for_one)
     - out of scope: distribution, preemptive scheduler, per-process GC, hot code swap
   - Define exact semantics for abnormal exit and linked-process propagation.

2. **Runtime core scaffolding**
   - Create focused runtime modules (keep files <500 LOC where practical):
     - `runtime/node.rs`
     - `runtime/process.rs`
     - `runtime/mailbox.rs`
     - `runtime/supervisor.rs`
     - `runtime/signal.rs`
   - Keep `runtime.rs` as façade/wiring entrypoint.

3. **Process and mailbox primitives (v1)**
   - Implement PID allocator + registry.
   - Implement spawn and async mailbox delivery.
   - Implement receive with timeout and stable timeout error value.
   - Add deterministic unit tests for spawn/send/receive/self behaviors.

4. **Links and monitors (v2 reliability core)**
   - Implement link table + monitor table.
   - Emit deterministic DOWN/EXIT notifications.
   - Ensure linked abnormal exits trigger configured propagation.
   - Add tests for normal exit vs abnormal exit behavior differences.

5. **Supervisor and restart intensity**
   - Implement one_for_one supervisor event loop.
   - Add restart policy and intensity-window enforcement.
   - Propagate escalated failure when intensity exceeded.
   - Add integration tests covering child crash, restart, and escalation.

6. **Gen-server-like request/response pattern**
   - Implement minimal behavior helper to process `call` and `cast` messages with state transitions.
   - Add timeout handling and caller-side error semantics.
   - Add integration fixtures demonstrating stable request-response loops.

7. **Backpressure and fairness hardening**
   - Enforce bounded mailbox capacities and explicit overflow behavior.
   - Add fairness guardrails (yield points in process loops); if LLVM backend path exists, define instrumentation hook now even if initially no-op.
   - Add load-oriented tests to validate overflow handling and non-starvation assumptions within practical CI limits.

8. **Language/IR integration path**
   - Expose OTP-lite primitives through runtime builtins first (or stdlib wrappers) without requiring full syntax additions in same slice.
   - If syntax changes are needed, gate them behind minimal parser/IR additions and targeted tests.

9. **Docs and migration notes**
   - Document operational behavior, caveats, and non-goals.
   - Add examples under `examples/` for:
     - supervised worker crash recovery
     - monitor-based health watcher
     - bounded-mailbox overload handling

## Acceptance Criteria

1. **Process Isolation and Messaging**
   - Given two spawned processes with unique PIDs
   - When one sends a message to the other and the receiver calls `receive`
   - Then the receiver obtains the message in mailbox order without mutating sender state

2. **Link Failure Propagation**
   - Given two linked processes A and B
   - When A exits abnormally with an error reason
   - Then B receives deterministic linked-exit signal behavior consistent with configured OTP-lite semantics

3. **Monitor Observability**
   - Given process A monitors process B
   - When B exits for any reason
   - Then A receives a deterministic monitor notification including target PID and exit reason

4. **Supervisor One-for-One Restart**
   - Given a supervisor managing child worker W with restart policy `permanent`
   - When W crashes abnormally
   - Then the supervisor restarts W and records restart metadata

5. **Restart Intensity Escalation**
   - Given a supervisor with restart intensity limit (N restarts in T seconds)
   - When a child exceeds the allowed restart frequency
   - Then the supervisor stops restarting and returns an escalation failure with deterministic diagnostics

6. **Bounded Mailbox Backpressure**
   - Given a mailbox capacity limit C
   - When senders enqueue beyond C under sustained load
   - Then overflow behavior follows the documented contract (`err(:mailbox_full)` or blocking), without unbounded memory growth

7. **Request/Response Timeout Semantics**
   - Given a gen-server-like process handling synchronous `call`
   - When a response is not produced within timeout
   - Then the caller receives a timeout error and the runtime remains healthy

8. **Cooperative Fairness Guardrail**
   - Given CPU-heavy process logic in OTP-lite runtime loops
   - When the process runs alongside other active processes
   - Then cooperative yielding prevents starvation of message handling within practical test thresholds

9. **Existing Runtime Behavior Regression Safety**
   - Given existing language fixtures for arithmetic, results, case, maps/tuples/keywords, and protocol dispatch
   - When the full test suite is run after OTP-lite integration
   - Then prior behavior remains correct and all existing contracts still pass

10. **No Hot Code Upgrade Commitment in MVP**
   - Given runtime documentation and user-facing behavior notes
   - When users inspect OTP-lite capabilities
   - Then hot code upgrade support is explicitly marked out-of-scope for this milestone

11. **Automated Coverage and Determinism**
   - Given the OTP-lite implementation
   - When `cargo test` is executed repeatedly in CI-like conditions
   - Then OTP-lite tests pass consistently without timing-flake dependence on arbitrary sleep-based races

## Metadata
- **Complexity**: High
- **Labels**: Runtime, OTP-lite, Concurrency, Reliability, Supervision, Backpressure, LLVM-Adjacent
- **Required Skills**: Rust async runtime design, actor-model semantics, failure modeling, integration testing, interpreter/runtime integration