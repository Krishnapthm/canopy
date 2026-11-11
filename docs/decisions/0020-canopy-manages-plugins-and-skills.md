# 0020 - Canopy manages plugins and skills

Status: Accepted (2026-11)

## Context

Every canopy-launched agent loads the user's plugins and skills, so their context and start-up cost is paid in every node of the tree. Each vendor manages them in its own place with its own commands. Until now canopy treated them as the user's and never changed harness config. The user decided canopy should be the one place to see and switch them.

## Decision

- Canopy lists and toggles plugins and skills for every supported harness, from the TUI and the web.
- Scopes are global and per project. There is no per-agent selection: an agent gets what its harness and project have switched on.
- Each item has one switch per harness, so a plugin can be on for Codex and off for Claude.
- Changes go through the vendor's own interface: `claude plugin enable|disable --scope`, and for Codex the app-server `config/value/write` and `skills/config/write` methods. Canopy does not hand-edit vendor settings files.
- Skills have one shared library owned by canopy in `~/.canopy/skills/`, one `SKILL.md` folder per skill. A library skill is exposed to a harness by a symlink in that harness's skill directory; switching it off for that harness removes the link. This link is the one thing canopy writes inside a vendor directory.
- A skill that already lives in a vendor directory is listed. Canopy toggles it where the vendor has a switch (Codex) and otherwise shows it read-only until the user adopts it into the library.
- Every change is an explicit user action on one named item. Nothing is ever auto-accepted: any command a marketplace declares is shown in full and needs a yes each time.
- Installing a plugin is not done by canopy in the first version; it shows the vendor's install command.
- A change applies to sessions started after it. Running agents keep what they loaded, and the UI says so.
- State is never cached: each view reads the vendor's current answer, so canopy and the vendor cannot disagree.

## Consequences

Canopy now changes state outside `~/.canopy`. The earlier rule "nothing inside an agent" narrows to "nothing inside a running agent". Tests still never touch the real harness configs: they run against fake CLIs in a temp home. Codex's app-server plugin methods are marked under development upstream, so the adapter must tolerate their absence.

## Rejected

Hand-editing `~/.claude` and `~/.codex` files: undocumented internals that drift weekly; kept as a later feasibility check only. A per-agent plugin set: more to configure for a need nobody has shown yet. A canopy plugin format: the vendors' formats differ and canopy would have to translate both.
