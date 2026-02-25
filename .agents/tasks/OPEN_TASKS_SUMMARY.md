# Open Tasks Summary

Current task sets:

1. `.agents/tasks/2026-02-25-tonicctl-meta-tool/` (10-task sequence)
   - Status: all tasks `pending` (`HEARTBEAT_TASK_STATUS: todo`)
   - Focus: evolve `examples/apps/tonicctl` from a pure planner into an executable meta-tool for tonic.

2. `.agents/tasks/2026-02-24-llvm-catalog-parity/` (9-task sequence)
   - Status: all tasks `completed` (`HEARTBEAT_TASK_STATUS: done`)
   - Focus: drive `tonic compile <path>` executable output + direct executable runtime to full parity with `examples/parity/catalog.toml`.
   - Baseline snapshot: compile expectation match `64/64`, runtime parity `62/62`.

3. `.agents/tasks/2026-02-24-native-compiler-roadmap/` (14-task sequence)
   - Status: all tasks `completed` (`HEARTBEAT_TASK_STATUS: done`)
   - Focus: LLVM-backed native AOT path, differential correctness/fuzzing, optimization, and CI competitive gates.

## Incomplete Task Count

- Pending tasks: **10**
- Completed tasks (active sets): **23**

*(Superseded scopes such as OTP-lite and reliability umbrella tasks remain excluded from active tracking.)*
