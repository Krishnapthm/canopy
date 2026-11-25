# Terminal appearance

2026-11-25, macOS. Ran the installed CLIs in a temporary PTY with empty
configuration directories and no credentials, prompts, or model turns.

- **Claude Code 2.1.331** queried the default background with OSC 11 and
  subscribed to color-scheme updates with DEC mode 2031 before onboarding.
- **Codex CLI 0.171.0** queried the default foreground and background with
  OSC 10 and 11 before sign-in. It did not subscribe to mode 2031 during the
  probe. Restarting Codex makes it query the current palette on startup.

Canopy's terminal test verifies OSC 11, color-scheme queries, subscribed
changes, and unsubscribing against the pinned Ghostty engine. The offline
probe establishes what each CLI sends before sign-in. Redrawing an existing,
signed-in agent session was outside the verification scope.
