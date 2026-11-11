# Terminal rendering

UNVERIFIED: single-cell rendering in Terminal.app, iTerm2, Ghostty and tmux needs
an interactive test card. No visible terminal is available in this implementation
session. Default symbols use text presentation; font detection is not attempted.
CANOPY_NERD_FONT explicitly selects a Nerd Font earth glyph. The default scope
symbol is a circled plus fallback until a single-cell continent globe is verified.

First paint occurs before connecting or detection. A test backend can measure the
renderer, but cannot establish real terminal startup latency or idle process CPU.
Those budgets need a PTY process measurement on the target machine.
