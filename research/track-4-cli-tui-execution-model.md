# Research: Track 4 — CLI/TUI Execution Model and Libraries

## Summary

Building a CLI/TUI subsystem for a new language requires decisions across five
interlocking layers: the **event loop model**, **terminal rendering engine**,
**key/input handling**, **subprocess orchestration**, and **cross-platform
abstraction**. The most successful ecosystems (Rust's Ratatui, Go's Bubble Tea,
Node's Ink, Python's Textual) converge on two architectural patterns — the **Elm
Architecture (TEA)** for application logic and **immediate-mode rendering with
double-buffered cell-grid diffing** for output — but diverge sharply on async
concurrency models. The idiomatic API for a new language should expose a unified
`Program` abstraction with declarative command definitions, a
`model / update / view` lifecycle for interactive TUIs, and structured async
subprocess pipelines.

---

## Findings

### 1. Event Loop Models

**Three dominant patterns:**

- **Cooperative single-threaded (Node/libuv style):** A single event loop
  multiplexes I/O via epoll/kqueue/IOCP. All handlers run sequentially;
  long-running work is offloaded to a thread pool. Simple reasoning, but
  blocking the loop stalls everything.
  [Node.js Event Loop](https://nodejs.org/en/learn/asynchronous-work/event-loop-timers-and-nexttick)

- **M:N green threads (Go runtime / Tokio style):** A work-stealing scheduler
  maps many goroutines/tasks onto N OS threads. Tokio uses `mio` (epoll/IOCP
  abstraction) as its reactor layer; Go's runtime integrates network poller
  directly. Both support true parallelism on multi-core without explicit thread
  management.
  [Tokio GitHub](https://github.com/tokio-rs/tokio) |
  [mio](https://github.com/tokio-rs/mio)

- **Actor + process model (Elixir/Erlang):** Each process has a mailbox; the
  scheduler is preemptive with reduction counting. No shared memory; all
  communication is message passing. Ratatouille TUIs run as supervised OTP
  processes, making crash recovery and hot-reload natural.
  [Ratatouille](https://github.com/ndreynolds/ratatouille)

**Recommendation for new language:** A cooperative async model (futures/tasks)
with a pluggable multi-threaded executor gives the best balance. The event loop
should be the runtime's default entry point — no boilerplate `asyncio.run()` or
`#[tokio::main]` annotations needed.

---

### 2. Terminal Rendering: Immediate Mode + Double-Buffer Diffing

**Retained mode** (traditional GUI): the library owns the widget tree and
redraws changed subtrees. Hard to reason about; harder to compose.

**Immediate mode**: every frame, the app re-describes the entire UI. The
renderer computes what to emit.

The winning pattern in terminal work combines both:

- App uses **immediate-mode API** (describe the full UI each tick, no retained
  state in the renderer).
- Renderer maintains **two cell-grid buffers** (current + previous). Each cell
  stores character, foreground/background color, style flags.
- After rendering, the renderer **diffs the two buffers** and emits only the
  ANSI escape sequences needed to update changed cells.
- This yields minimal terminal writes — typically a few dozen bytes per frame
  even for large UIs.

[Ratatui rendering architecture](https://ratatui.rs/concepts/rendering/under-the-hood/) |
[FrankenTUI diff-based kernel](https://github.com/Dicklesworthstone/frankentui)

**Ratatui's concrete approach:**

```
Frame render (app code)
  └─> writes into Buffer (cell grid)
      └─> diff: Buffer::diff(prev, curr) → Vec<(x, y, cell)>
          └─> Backend::draw(diffs) → ANSI escape writes to stdout
```

**Ink (Node.js)** uses a React custom renderer (`react-reconciler`) backed by
Facebook's Yoga flexbox engine for layout, then renders the virtual DOM tree to
terminal output strings.
[Ink GitHub](https://github.com/vadimdemedes/ink) |
[Ink reconciler source](https://github.com/vadimdemedes/ink/blob/master/src/reconciler.ts)

**Textual (Python)** adds CSS-driven layout on top of Rich's markup engine,
treating widgets like browser DOM nodes with a subset of CSS properties. The
async event loop (asyncio) drives both rendering and I/O.
[Textual](https://textual.textualize.io/)

---

### 3. The Elm Architecture (TEA) — Universal TUI Pattern

All major TUI frameworks independently converge on TEA:

```
Model  — immutable snapshot of all application state
Update — pure function: (Model, Msg) → (Model, Cmd)
View   — pure function: Model → UI description
Cmd    — side-effect description (I/O, subprocess, timer) returned from Update
```

The runtime loop:
1. Call `view(model)` → render to terminal
2. Wait for next event (keypress, timer, subprocess result, resize)
3. Wrap event as a `Msg`, call `update(model, msg)` → new model + optional `Cmd`
4. Execute `Cmd` asynchronously; when done, produce another `Msg`
5. Repeat

**Why TEA works for terminals:** Terminals are inherently event-driven and
stateful, but TEA makes state transitions explicit and testable. `update` is a
pure function — trivial to unit test without a terminal.

| Ecosystem | Framework | TEA? | Notes |
|-----------|-----------|------|-------|
| Go | Bubble Tea | ✅ | `Init/Update/View` interface; `tea.Cmd` for side effects |
| Rust | Ratatui | Optional | DIY or use `tui-realm` / `tuirealm` |
| Node | Ink | ❌ | React component model instead |
| Python | Textual | Partial | Event handlers on widgets; workers for async |
| Elixir | Ratatouille | ✅ | `init/1`, `update/2`, `render/1` callbacks |
| Elixir | Owl | Partial | Simpler: live-updating output blocks |

[Bubble Tea GitHub](https://github.com/charmbracelet/bubbletea) |
[Ratatui TEA pattern](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/) |
[Ratatouille](https://github.com/ndreynolds/ratatouille)

---

### 4. Key Input Handling: Raw Mode Pipeline

**Raw mode** disables the terminal driver's line buffering and echo, giving the
process every keypress byte immediately.

**Pipeline:**
1. Save terminal state (`tcgetattr` on Unix / `GetConsoleMode` on Windows)
2. Set raw mode (`cfmakeraw` / `SetConsoleMode` with flags cleared)
3. Read bytes from stdin (blocking or via async select/epoll)
4. Parse ANSI escape sequences:
   - Regular chars: `0x20`–`0x7E`
   - Control chars: `0x01`–`0x1F` (e.g., `0x03` = Ctrl+C, `0x0D` = Enter)
   - Escape sequences: `ESC [` + params + final byte (arrow keys, F-keys, etc.)
   - Mouse events: `ESC [ M` or `ESC [ <` (SGR mouse protocol)
5. Normalize into structured key events: `{key: "ArrowUp", modifiers: ["ctrl"]}`
6. Restore terminal state on exit (RAII / defer / finally)

[ANSI escape code reference](https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797) |
[Ink raw mode + parseKeypress](https://deepwiki.com/vadimdemedes/ink/7.3-raw-mode-and-input-processing) |
[Li Haoyi ANSI tutorial](https://www.lihaoyi.com/post/BuildyourownCommandLinewithANSIescapecodes.html)

**Modifier detection** (Shift/Alt/Ctrl) is often ambiguous in terminal escape
sequences. The **Kitty keyboard protocol** (`CSI u`) solves this with
unambiguous encoding — increasingly supported in modern emulators (Kitty,
WezTerm, foot). A good library should detect and use it when available with
graceful fallback.

**Mouse input:** Enable with `ESC [ ? 1000 h` (normal) or `ESC [ ? 1006 h`
(SGR extended, supports >223 columns). Track position, button, scroll wheel,
drag events.

---

### 5. Subprocess Orchestration

**Core primitives needed:**

| Operation | Unix | Windows |
|-----------|------|---------|
| Spawn | `fork`/`exec` or `posix_spawn` | `CreateProcess` |
| Stdio | `pipe(2)` → fd pairs | `CreatePipe` / `HANDLE` |
| PTY (interactive) | `openpty`/`forkpty` | ConPTY (`CreatePseudoConsole`) |
| Wait | `waitpid` | `WaitForSingleObject` |
| Signal | `kill(pid, SIGTERM)` | `TerminateProcess` / `GenerateConsoleCtrlEvent` |

**Async subprocess pattern (Python asyncio as reference):**
```python
proc = await asyncio.create_subprocess_exec(
    "cargo", "build",
    stdout=asyncio.subprocess.PIPE,
    stderr=asyncio.subprocess.PIPE,
)
async for line in proc.stdout:
    # stream output as it arrives
    print(line.decode())
await proc.wait()
```
[Python asyncio subprocess docs](https://docs.python.org/3/library/asyncio-subprocess.html)

**Swift Subprocess** (new in 2024) is a notable design reference — clean async
API with structured concurrency, `for try await line in standardOutput.lines()`:
[Swift Subprocess CLI](https://www.swift.org/get-started/command-line-tools/)

**Shell pipelines vs structured pipelines:** Most languages model `cmd | cmd`
as connecting stdout→stdin pipes. A new language can offer a higher-level
`Pipeline` type where each stage is a typed async stream, enabling:
- Compile-time detection of broken pipes
- Structured error propagation (stderr as a separate typed stream)
- Backpressure across stages

**PTY vs pipe:** Use pipes for non-interactive processes (build tools, scrapers).
Use a PTY when the subprocess needs to believe it's connected to a terminal
(testing CLI tools, spawning shells, multiplexers). ConPTY on Windows (added in
Win10 1809) makes this cross-platform.
[ConPTY announcement](https://devblogs.microsoft.com/commandline/windows-command-line-introducing-the-windows-pseudo-console-conpty/)

---

### 6. Cross-Platform Constraints

| Concern | Unix | Windows |
|---------|------|---------|
| Raw mode API | `termios.h` (`cfmakeraw`) | `SetConsoleMode` |
| ANSI support | Universal | Win10+ (VT processing flag required) |
| PTY | `/dev/pts/*`, `openpty` | ConPTY (`CreatePseudoConsole`) |
| Terminal size | `TIOCGWINSZ` ioctl | `GetConsoleScreenBufferInfo` |
| Resize signal | `SIGWINCH` | `SetConsoleCtrlHandler` + polling |
| True color | Widespread | WT/ConPTY only |
| Unicode | UTF-8 | UTF-16 internally; code page matters |

**Key constraint:** Windows terminals before Win10 1809 have no ConPTY and very
limited VT support. `crossterm` (Rust) is the gold standard for cross-platform
terminal abstraction — it handles all the above divergences behind a single API.
[crossterm GitHub](https://github.com/crossterm-rs/crossterm)

**Double-width characters (CJK, emoji):** A cell grid must handle multi-column
glyphs. The `unicode-width` algorithm (UAX #11) determines column count per
codepoint. Zero-width joiners and variation selectors complicate emoji sequences.

---

### 7. CLI Argument Parsing API Design

Mature ecosystems reveal two philosophies:

**Declarative/struct-driven (Clap Rust):** Parse target is a struct; derive
macros generate the parser. Type-safe, composable, auto-generates `--help`.
```rust
#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}
```
[Clap docs](https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html)

**Decorator/annotation-driven (Click Python, Cobra Go):** Commands are
functions with annotations/decorators. More ergonomic for scripting; less
type-safe.
```python
@click.command()
@click.option('--count', default=1)
@click.argument('name')
def hello(count, name):
    ...
```

**For a new language:** Struct/annotation-driven with type inference wins. The
parser should be derivable from the type signature of the command handler
function itself — no separate schema. Pattern:

```
command build(target: Path, --release: Bool = false, --jobs: Int = 4) {
  // implementation
}
```
The language generates: argument parsing, `--help`, type coercion, error
messages, shell completion scripts.

---

## Proposed Idiomatic API Shape

### CLI Commands

```
// Simple command — handler signature IS the schema
command greet(name: String, --loud: Bool = false) {
  let msg = if loud { name.upper() } else { name }
  print("Hello, {msg}!")
}

// Subcommands via sum type
command git {
  subcommand commit(message: String, --amend: Bool = false) { ... }
  subcommand push(remote: String = "origin", branch: String = "HEAD") { ... }
}

// Built-in: --help, --version, shell completion, man page generation
```

### Subprocess Orchestration

```
// Inline shell expression — pipeline of structured streams
let lines = run("cargo build --release") | lines()

// Async streaming
for await line in run("tail -f /var/log/app.log").stdout {
  if line.contains("ERROR") { alert(line) }
}

// Structured pipeline — typed stages
let result = run("find . -name '*.rs'")
  | pipe { run("wc -l") }
  | collect()

// PTY for interactive subprocesses
let shell = spawn_pty("/bin/bash")
shell.send("ls -la\n")
```

### TUI Programs

```
// Elm Architecture baked in
program Counter {
  // State
  model: { count: Int, running: Bool }

  // Initial state + startup commands
  init() -> (Model, [Cmd]) {
    ({ count: 0, running: true }, [tick(1000ms)])
  }

  // Pure state transition
  update(model: Model, msg: Msg) -> (Model, [Cmd]) {
    match msg {
      .increment => ({ ...model, count: model.count + 1 }, [])
      .quit      => ({ ...model, running: false }, [Cmd.quit])
      .tick      => (model, [fetch_count(), tick(1000ms)])
    }
  }

  // Pure render — immediate mode, returns layout tree
  view(model: Model) -> View {
    vstack {
      text("Count: {model.count}", style: .bold)
      hstack {
        button("[+]", on: .increment)
        button("[q]uit", on: .quit)
      }
    }
  }
}
```

### Key Bindings

```
// Declarative keybindings on views
on_key {
  "ctrl+c", "q" => .quit
  "up", "k"     => .move_up
  "down", "j"   => .move_down
  "enter"       => .select
}
```

### Layout Primitives

The view layer should expose composable layout nodes (inspired by Yoga/flexbox):
- `vstack`, `hstack` — vertical/horizontal flex containers
- `text(content, style)` — styled inline text
- `block(title, border_style)` — bordered container
- `list(items, selected)` — scrollable list with cursor
- `table(rows, columns)` — data table
- `progress(value, max)` — progress bar
- `input(placeholder, value)` — text input with cursor

---

## Sources

**Kept:**
- [Ratatui rendering internals](https://ratatui.rs/concepts/rendering/under-the-hood/) — authoritative on double-buffer diff approach
- [Ratatui TEA pattern](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/) — architecture reference
- [Bubble Tea GitHub](https://github.com/charmbracelet/bubbletea) — canonical Go TEA TUI; widely adopted
- [Ink GitHub + reconciler](https://github.com/vadimdemedes/ink) — React custom renderer for terminal; production-grade
- [Ink raw mode + parseKeypress](https://deepwiki.com/vadimdemedes/ink/7.3-raw-mode-and-input-processing) — detailed key input pipeline
- [Textual](https://textual.textualize.io/) — modern Python async TUI; CSS widgets
- [Ratatouille (Elixir)](https://github.com/ndreynolds/ratatouille) — Elixir TEA TUI, OTP-native
- [Clap derive tutorial](https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html) — gold standard derive-based CLI parsing
- [ConPTY announcement](https://devblogs.microsoft.com/commandline/windows-command-line-introducing-the-windows-pseudo-console-conpty/) — Windows cross-platform PTY context
- [Python asyncio subprocess](https://docs.python.org/3/library/asyncio-subprocess.html) — async subprocess pattern reference
- [ANSI escape code reference](https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797) — comprehensive escape sequence spec
- [Li Haoyi ANSI tutorial](https://www.lihaoyi.com/post/BuildyourownCommandLinewithANSIescapecodes.html) — raw mode + key reading walkthrough
- [FrankenTUI](https://github.com/Dicklesworthstone/frankentui) — minimal diff-based render kernel; good for studying core primitives
- [BubbleTea vs Ratatui comparison](https://www.glukhov.org/post/2026/02/tui-frameworks-bubbletea-go-vs-ratatui-rust/) — recent architectural comparison
- [Swift Subprocess CLI](https://www.swift.org/get-started/command-line-tools/) — clean async subprocess API design reference
- [Tokio GitHub](https://github.com/tokio-rs/tokio) — M:N async runtime reference

**Dropped:**
- Stack Overflow ANSI/Windows answers — too narrow/situational
- Python Click vs argparse blog posts — redundant given Clap analysis
- LogRocket 7-TUI-libraries roundup — surface level survey, no depth
- DevOps language ranking articles — off-topic

---

## Gaps

1. **Kitty keyboard protocol** specifics — the unambiguous key encoding protocol
   (CSI u) is increasingly important for modifier detection but coverage in
   major framework docs is sparse. Worth a dedicated investigation:
   `site:sw.kovidgoyal.net keyboard-protocol`

2. **Double-width / emoji rendering correctness** — `unicode-width` edge cases
   (ZWJ sequences, variation selectors, regional indicators) are poorly
   documented outside Rust's `unicode-width` crate source. Needs empirical
   testing across terminal emulators.

3. **Inline TUI mode** (non-fullscreen, below cursor) — Bubble Tea and
   FrankenTUI both support this but the design constraints (scroll region,
   cursor save/restore, height negotiation) aren't well-documented. Worth
   studying Bubble Tea's `AltScreen` vs inline mode source.

4. **Shell completion generation** — a production CLI library must generate
   bash/zsh/fish/PowerShell completions. Clap's `clap_complete` crate is the
   best reference. Not yet investigated for dynamic completions (completions
   that call back into the app).

5. **Accessibility** — terminal screen reader support (via `TERM_PROGRAM` hints
   or explicit `$NVDA` env vars) is essentially an open problem. No major TUI
   framework handles this well.
