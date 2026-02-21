# Tonic Runtime Research: OTP-lite and Reliability Semantics

This document outlines the findings for building an Elixir-like language runtime (`tonic`) leveraging LLVM for codegen in Rust, specifically focusing on the minimal viable subset of OTP/BEAM reliability properties.

## 1. Feature Matrix: Semantics vs. Implementation

When designing a BEAM-inspired runtime, it is critical to separate the **language semantics** (what the programmer relies on) from the **VM implementation details** (how the BEAM achieves it under the hood). 

| Feature | Type | Operational Importance | Complexity (Rust/LLVM) | Feasibility (3-6 Months, Small Team) |
| :--- | :--- | :--- | :--- | :--- |
| **Lightweight, Isolated Processes** | Semantics | Foundational for fault isolation and concurrency scaling. | Low-Med | **Yes**. Rust's async tasks (`tokio::spawn`) map cleanly to this. |
| **Asynchronous Message Passing** | Semantics | Decouples components, allows non-blocking inter-process communication. | Low | **Yes**. Standard async channels (e.g., `tokio::sync::mpsc`) suffice. |
| **Links & Monitors** | Semantics | Essential primitives for detecting and propagating failures. | Medium | **Yes**. Can be implemented by tracking task join handles and cancellation tokens. |
| **Supervision Trees** | OTP Framework | "Let it crash" philosophy; automated recovery from transient failures. | Medium | **Yes**. Feasible to build a simplified `supervisor` over links/monitors. |
| **Preemptive Scheduling (Reductions)** | VM Implementation | Ensures low tail latency and fairness; prevents CPU-bound tasks from starving I/O. | Very High | **No**. Userland preemption in Rust is notoriously difficult. Rely on cooperative yielding. |
| **Per-Process Garbage Collection** | VM Implementation | Prevents system-wide Stop-The-World (STW) pauses; isolates memory leaks. | High | **No**. Requires a custom memory allocator per task. MVP should use standard Rust allocation. |
| **Hot Code Reloading** | VM Implementation | Enables zero-downtime stateful upgrades. | Very High | **No**. Extremely complex with AOT compilation (LLVM). |

---

## 2. Proposed Staged Roadmap for OTP-Lite

To deliver a working runtime in 3-6 months, the MVP must aggressively cut scope while retaining the core "feel" of Erlang fault tolerance.

### Stage v1: Concurrency Primitives
*   **Goals:** Spawning isolated async processes, unique PIDs, and basic asynchronous message passing (mailboxes).
*   **Deliverables:** Process registry, `spawn/1`, `send/2`, and basic `receive`.
*   **Non-Goals:** Preemption, cross-node distribution, fault recovery, hot code reloading.

### Stage v2: Reliability Primitives (The "OTP-Lite" MVP)
*   **Goals:** Provide the tools to detect and recover from crashes.
*   **Deliverables:** 
    *   **Links** (bidirectional failure propagation).
    *   **Monitors** (unidirectional failure observation).
    *   **Simplified `gen_server`**: A standard behavior for stateful processes.
    *   **Basic Supervisors**: `one_for_one` restart strategy.
*   **Non-Goals:** Advanced restart strategies (`rest_for_one`, `one_for_all`), application lifecycle management, release packaging.

### Stage v3: Hardening & Backpressure
*   **Goals:** Prevent system overload and OOM errors under heavy load.
*   **Deliverables:** Bounded mailboxes with synchronous `call` timeouts, explicit rate limiting, task yielding in codegen.
*   **Non-Goals:** Transparent distributed clustering (Erlang Distribution Protocol).

---

## 3. Failure Model Recommendations

For the MVP, replicate the core Erlang failure model using Rust's async primitives:

*   **Links and Monitors:** Implement links as paired cancellation tokens. If Task A panics or returns an `Err`, its linked tasks receive a cancellation signal. Monitors can be implemented by holding a weak reference to a task's join handle and awaiting its completion.
*   **Supervision:** A supervisor is simply an async task that spawns children and loops, `select!`ing on their join handles. When a child exits, the supervisor evaluates the restart intensity (e.g., max 3 restarts in 5 seconds) and respawns it.
*   **Mailbox & Backpressure:** 
    *   *BEAM Default:* Unbounded mailboxes (risk of OOM).
    *   *Tonic Recommendation:* **Bounded mailboxes** (`mpsc::channel` with a generous limit, e.g., 10,000). If the queue fills, `send` should return an error or block (applying implicit backpressure), forcing developers to handle overload explicitly rather than crashing the whole node.
    *   Leverage synchronous `call` (which waits for a response with a timeout) as the primary backpressure mechanism for client-server interactions.

---

## 4. Scheduler / Process Model Tradeoffs

Building an actor model in Rust presents a distinct architectural choice:

*   **Option A: Custom Scheduler with Preemption (Like BEAM)**
    *   *Pros:* Perfect fairness; true BEAM semantics.
    *   *Cons:* Immense engineering effort. Requires saving/restoring CPU registers in user space. Incompatible with the broader Rust async ecosystem.
*   **Option B: Tokio Tasks (Cooperative Scheduling)**
    *   *Pros:* Off-the-shelf, battle-tested, highly performant for I/O, integrates seamlessly with existing Rust libraries.
    *   *Cons:* Susceptible to starvation. A `while(true)` loop in Tonic code that never awaits will lock up a Tokio worker thread.
    *   *Recommendation for MVP:* **Use Tokio.** To mitigate CPU starvation, the LLVM codegen should inject cooperative yield points (e.g., `tokio::task::yield_now()`) at the end of loops or function prologues, simulating BEAM's "reductions."

---

## 5. Hot Code Upgrade Stance for MVP

**Stance: Explicit Non-Goal.**

The BEAM achieves hot code swapping because it is essentially an interpreter (or a tracing JIT) that resolves function calls dynamically. When `Module:Function` is called, it looks up the latest loaded bytecode. 

For an LLVM-compiled, statically-linked binary, achieving this requires complex dynamic library loading (`dlopen`), ABI stability guarantees, and intricate state migration hooks (`code_change/3`) to map old structs to new structs in memory. 

For the MVP, rely entirely on modern infrastructure (Kubernetes, Docker) for **rolling deployments**. When code changes, the process restarts.

---

## 6. High-Quality Sources

1.  **The Erlang Runtime System (ERTS) Architecture:** Details the internal workings of the BEAM, including memory management and process isolation. [Erlang Docs](https://www.erlang.org/doc/apps/erts/erts.html)
2.  **Learn You Some Erlang - Errors and Processes:** Comprehensive guide on Links, Monitors, and the "Let it Crash" philosophy. [LearnYouSomeErlang](https://learnyousomeerlang.com/errors-and-processes)
3.  **Learn You Some Erlang - Designing a Concurrent Application:** Breaks down the implementation of supervision trees. [LearnYouSomeErlang](https://learnyousomeerlang.com/designing-a-concurrent-application)
4.  **Firefly (formerly Lumen) Architecture:** A Rust-based compiler/runtime for Erlang targeting WebAssembly/LLVM. Demonstrates how to map BEAM semantics to LLVM IR and Rust runtimes. [Firefly GitHub repo](https://github.com/GetFirefly/firefly)
5.  **Lunatic Framework:** An Erlang-inspired runtime for WebAssembly in Rust, showcasing timer-based cooperative scheduling vs BEAM reductions. [Lunatic](https://lunatic.rs/)
6.  **Tokio Tutorial - Spawning:** Discusses the cooperative nature of Tokio tasks and the dangers of blocking the executor. [Tokio Docs](https://tokio.rs/tokio/tutorial/spawning)
7.  **Erlang In Anger - Overload and Backpressure:** Fred Hebert's definitive guide on why unbounded mailboxes fail and how to apply backpressure in OTP. [Erlang in Anger (PDF/Web)](https://www.erlang-in-anger.com/)
8.  **The BEAM Book:** In-depth, low-level explanation of the BEAM VM, reductions, and the scheduler. [GitHub/The BEAM Book](https://github.com/happi/theBeamBook)
9.  **OTP Design Principles:** Official documentation on Supervisors, GenServers, and Applications. [Erlang OTP Docs](https://www.erlang.org/doc/design_principles/des_princ.html)
10. **Hot Code Loading in Erlang:** Explanation of the two-version module system and `code_change`. [AppSignal Blog](https://blog.appsignal.com/2021/08/24/how-to-do-hot-code-swapping-in-elixir.html)
11. **Why Erlang/Elixir Mailboxes are Unbounded:** Discussion on the rationale and risks of unbounded queues in actors. [Ferd.ca](https://ferd.ca/handling-overload.html)
12. **sched-ext: Rust-ifying the Linux Kernel Scheduler:** Context on why userland preemption is hard and kernel-level approaches in Rust. [LWN.net/Kernel](https://lwn.net/Articles/922405/)
