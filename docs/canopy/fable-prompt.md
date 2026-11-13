# Prompt for Fable: Canopy connections, canvas feel and keyboard

You are planning the next stretch of work on Canopy, a native Rust/GPUI
terminal multiplexer whose surface is a free canvas. Colored "spaces" hold
cells, and each cell is a terminal or a coding agent (Claude Code, Codex and
others) running in a PTY. Before anything else, read AGENTS.md,
docs/canopy/plan.md, docs/canopy/herdr-map.md, docs/canopy/agent-integration.md
and docs/canopy/harness-notes/. Branch: `pivot/canvas`.

**Your job is a plan, not code.** Do not edit source. You have creative
liberty. Section 0 lists decisions I have already made. Everything else
describes what I want to feel and achieve, not a spec. Where my wording
hints at a mechanism, you are free to beat it, and you can question my
premises if you see a better shape. Bold, simple ideas are welcome. I want
the least complexity that fully delivers the experience. Spawn small, cheap
sub-agents (Haiku) freely for research: reading this repo, reading the
installed CLIs and their docs, and surveying how other tools solve the same
problems. Read-only experiments are fine.

The output is a plan that other sessions will implement over several
sittings. It should have ordered phases. For each phase, give:

- the goal
- the design decisions and why
- what you ruled out and why
- risks and open questions for me
- how I can check the phase by hand

---

## 0. Decisions already made (build on these)

- Connected agents talk to each other when I ask them to, and also on their
  own when the task genuinely needs the other agent. Connecting alone never
  triggers chatter.
- Connections survive restarts. When a connected agent is closed and later
  restarted or resumed, its edges stay and it is told about its connections
  again automatically.
- This plan covers communication **within a space** only. Cross-space
  communication is a later plan, but do not paint us into a corner.
- Claude's in-process sub-agents become **usable cells** that I can open,
  read and message. They are automatically connected to their parent.
- Terminals expand **inside the canvas**. This replaces today's side panel
  for terminals and agents. The side panel stays only for file previews.
- Cells never overlap. Neighbours move away **live, while dragging or
  resizing**. Cells stay inside their space, so the space grows to hold
  them and pushes other spaces away.
- Keyboard cell navigation uses a **prefix key** (like the herdr TUI's
  Ctrl+B): press, release, then the arrow keys move between cells until Esc
  or Enter.
- The focus key **toggles zoom**. It fits the current space to the screen,
  and pressing it again returns to the previous view. The fit accounts for
  the space the right-side file preview panel takes up when it is open.
- One-click arrange acts on the **current space** or on the **selection**.
- Config stays file-based and hand-editable. herdr-app already keeps
  `settings.json`, `keys.json` and `programs.json` in the app home
  (`~/.config/canopy`), watched and reloaded live (src/files.rs,
  src/settings.rs, src/persist.rs). The core has its own `config.toml`.
  New settings and shortcuts go into these files. Do not add a second
  config system.

Edge direction and who may message whom are yours to decide.

## 1. A bug to explain first

When Claude spawns sub-agents, they appear on the canvas as pills reading
"general-purpose" with an "idle" state, and nothing can be done with them.
Start around src/subs.rs, the SubagentStop hook setup in src/programs.rs and
src/window/canvas_view.rs. Find the root cause and reproduce it the way a
user would see it. Then decide whether the fix belongs in the current
sub-agent path or whether the connection layer in section 2 replaces that
path. Sub-agents must end up as usable, connected cells either way.

## 2. Connections between agents (the main event)

We already have a cross-agent, cross-vendor setup made of hooks, the `hc`
helper and sub-agent tracking. It feels heavy and fragile. I want something
lighter and more durable that keeps Canopy what it is: a terminal
multiplexer canvas, not an agent framework.

What I want to feel:

- **Connecting is drawing.** It should feel like a whiteboard or node
  editor. Hover a cell and a small round "+" appears at each of its four
  corners. Drag from one to another cell to connect them. One-to-many and
  many-to-one both work. The connection shows as an edge on the canvas.
- **Connecting only sets up the channel.** Each agent learns that the other
  exists and how to reach it, and then both get on with their own work.
- **No context dumping.** An agent never reads another agent's transcript or
  context wholesale. Messages are deliberate and scoped, and each agent's
  context stays its own.
- **We own the layer, and it stays vendor-agnostic.** No vendor APIs or
  credentials. Anything that works through what every CLI already has is
  fair game: files, terminals, instruction files, hooks, MCP, flags. That
  could be a shared file, a shared instruction file, a small message
  protocol, an MCP server that Canopy serves, PTY injection, or something
  nobody has tried yet. Explore and pick. I am open to inventing our own
  protocol if it is clearly better suited.
- **Zero cost when idle.** No polling agents, and no tokens spent just
  because a connection exists.

The research behind your decision should cover how Claude Code (sub-agents,
teams, SendMessage, hooks, MCP), Codex and other coding agents let agents
communicate today. Include multi-agent tools and terminal multiplexers built
around agents. For each approach, note what survives a restart and what it
costs in tokens and fragility. Verify against the installed CLIs, not
memory. Keep per-agent support as data (programs.json and the detect
manifests) wherever possible.

Decide these explicitly:

- what an edge means
- how an agent learns about a new or removed connection mid-session
- how a message arrives, and whether I can see it on the canvas
- what happens when the target is busy or not running
- how this relates to the existing quote-to-agent flow

## 3. Canvas feel

Everything should feel smooth. That matters more than any single feature.

- **Spacing.** The no-overlap rule should feel magnetic, like trees keeping
  their distance in a forest. Every cell keeps a consistent padding from its
  neighbours. Pick the mechanism (physics, constraint solving, something
  else) and tell me what good canvas tools do.
- **In-canvas terminals.** A cell expands in place into a live terminal with
  a sensible default size and can be resized. Nothing gets hidden or pushed
  behind it.
- **Arrange.** Propose the set of layouts worth having for one-click
  arrange: rows, columns, grid, or whatever canvas apps show is useful.
- **Quieter spaces.** The space background is too present. Use a thin
  border with no fill, or a very light fill around 5% opacity.

## 4. Keyboard first

Canopy should be fully usable from the keyboard, with shortcuts that are
robust and easy to learn. Design the full shortcut map: focus/zoom, prefix
navigation, expand and collapse, connect, arrange and new cell. Check that
it does not clash with keys that the terminals and agents inside cells
need. A focus control also sits at the top right. Shortcuts live in
`keys.json`.

## 5. Settings that earn their place

Expand what can be configured, but only where real people would choose
differently. Two examples of meaningful settings: whether neighbours move
while dragging or only on drop, and the spacing between cells. Compare our
config files with herdr's own (the TUI and herdr-core) and tell me if they
do something about hand-editing we should copy, such as documenting
defaults or reporting errors in the file. No option
should exist just to exist. For every setting you propose, give the
default and say who would change it and why. Everything stays editable by
hand in the config files.

## 6. Small UI changes

- Replace the quote icon with an actual quotation mark glyph, if one is
  available in our icon set or font.
- Remove the Agents / All / Terminals view switch from the top right, and
  keep only the file icon there. Move the view choice into the app's View
  menu. That is right for macOS. For Ubuntu, recommend the best equivalent,
  given that we draw our own chrome there.

---

## Constraints that do hold

These come from AGENTS.md:

- No telemetry and no app-owned network requests.
- Never touch the daily herdr app or its data.
- Use an isolated app home for any launch.
- No paid agent sessions without asking.
- Preserve released data and APIs unless the plan says it changes them, and
  then say so.
- Keep defaults/AGENTS.md and src/SKILL.md current with agent-facing
  behaviour.

macOS and Linux matter. Windows is not a target right now.

Everything else is yours to decide. Surprise me if you see a better way.
