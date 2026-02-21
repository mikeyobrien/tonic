Written to `/home/mobrienv/projects/tonic/.agents/planning/2026-02-20-elixir-tui-cli-language/research/raw/04-tui-cli-model.md` — 22 KB.

---

**What the document covers:**

**Event loop models** — Four distinct patterns catalogued with concrete code: Elm TEA (BubbleTea/Go, Ratatouille/Elixir, charm.clj/Clojure), tick-based loop with cancellation (Ratatui/Rust), async reactive component tree (Textual/Python), and React VDOM reconciliation (Ink/Node).

**Rendering & diffing** — Double-buffer cell diff (Ratatui's canonical approach), Textual's painter's algorithm + spatial-map for overlapping widgets, React VDOM diff (Ink), and a layout engine comparison (Ratatui manual Rects → Yoga flexbox → CSS lipgloss).

**Async IO** — Language event loop integration, Python asyncio subprocess streams, Go goroutines + `tea.Cmd`, Rust `tokio::select!` over crossterm `EventStream`.

**Key input** — Raw mode mechanics, escape sequence parsing, Kitty keyboard protocol, mouse events via DECSET, cross-platform table (crossterm vs termion vs Windows SetConsoleMode).

**Subprocess orchestration** — Pipe vs PTY decision tree, POSIX `openpty`, Windows ConPTY, signal propagation (SIGWINCH, SIGINT), and Nushell's structured pipeline model as inspiration for typed subprocess output.

**Cross-platform** — Full table covering ANSI support, PTY, raw mode, line endings, resize signals, colour depth, and Unicode per OS.

**Proposed API** — Five concrete API sketches: declarative struct-driven CLI commands, Elm-TEA TUI with typed `Cmd<Msg>`, three-function subprocess API (`run`/`stream`/`spawn_pty`), unified `Event` ADT with Kitty modifier support, and immediate-mode widget composition with Ratatui-style constraints.

**Gaps flagged:** focus trees, accessibility/screen readers, Kitty protocol negotiation fallback, web/WASM backend option, and tmux compatibility degradation.