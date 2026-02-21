Written to `/home/mobrienv/projects/tonic/.agents/planning/2026-02-20-elixir-tui-cli-language/research/raw/09-terminal-portability-gap.md` (~22KB).

**What's in it:**

- **PTY vs ConPTY** — documents the three concrete ConPTY failure modes (sequence rewriting, out-of-order delivery, missing XTGETTCAP), with linked GitHub issues and the Warp engineering post as primary sources.

- **tmux layer** — covers `$TERM` reset, `$COLORTERM` stripping, the `terminal-overrides` workaround, and the open issue with kitty keyboard passthrough.

- **SSH** — `$TERM` is forwarded automatically; `$COLORTERM` is not. XTGETTCAP in-band queries are the right answer for remote truecolor detection.

- **Color detection chain** — prioritized table from `NO_COLOR` → `TERM=dumb` → `isatty` → `COLORTERM` → `TERM` suffix → `XTGETTCAP` → terminfo. Notes macOS Terminal.app's no-truecolor limitation.

- **Unicode width** — ZWJ sequences, variation selectors, emoji skin tones, and CJK are all unreliable. Recommend avoiding emoji in UI chrome; use `ucs-detect` to catalogue actual discrepancies.

- **Mouse protocols** — SGR 1006 is the universal target; X10 breaks at >223 columns.

- **Keyboard protocols** — legacy is the safe v0 baseline; KKP is a progressive enhancement. macOS Terminal.app, PuTTY, and most CI contexts don't support it.

- **3-tier compatibility matrix** — Tier 1 (8 environments, CI-gated), Tier 2 (manual smoke), Tier 3 (best-effort).

- **v0 test matrix** — 8 specific environments with a per-environment checklist and ConPTY-specific checks.

- **Gaps section** — flags WSL, Elixir library audit, XTGETTCAP adoption reality, and ConPTY current-state as open questions.