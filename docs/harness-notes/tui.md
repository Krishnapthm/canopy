# Terminal rendering

UNVERIFIED: single-cell rendering in Terminal.app, iTerm2, Ghostty and tmux needs
an interactive test card. No visible terminal is available in this implementation
session. Default symbols use text presentation; font detection is not attempted.
CANOPY_NERD_FONT explicitly selects a Nerd Font earth glyph. The default scope
symbol remains the single-cell circled plus fallback. Unicode continent globes
U+1F30D through U+1F30F are emoji and measure two cells using ratatui
(unicode-width 0.2.3). There is no non-emoji Unicode continent globe; the
Nerd Font option supplies one. The width check is covered in tui/tree.rs.

First paint occurs before connecting or detection. A test backend can measure the
renderer, but cannot establish real terminal startup latency or idle process CPU.
Those budgets need a PTY process measurement on the target machine.

Mouse capture uses crossterm 0.29's standard SGR mouse reporting. Layout records
rectangles each paint; the last matching target wins so forms and toasts cover
underlying panes. Restore disables capture on normal exit and panic unwinding.
Interactive mouse reporting across terminals remains unverified here.

Missing Claude/Codex installation hints were checked on 2026-11-11 against
https://support.claude.com/en/articles/14552382-your-first-day-in-claude-code
and https://www.npmjs.com/package/@openai/codex. They are displayed only, never run.
