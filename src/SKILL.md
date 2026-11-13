# canopy: drive the person's window

Canopy shows spaces, tabs and panes. herdr-core (`hc`) runs their terminals
and keeps them alive when the window closes. Use `canopy` only to change
what the person sees. For starting, reading, messaging, waiting on or cleaning
up agents and programs, read `hc --skill` and use hc.
Your terminal address is `$HC_TERMINAL_ID`; it is not a pane ID.

## Inspect before acting

```sh
canopy ls          # spaces > tabs > terminals, including pane IDs
canopy status      # current layout, focus, overlays, zoom and file/mod errors
canopy commands    # available commands, arguments and descriptions
```

`ls` and `status` talk to the running window, not a new app. `commands`
prints the built-in command list even without an open window. Do not launch bare
`canopy` to inspect it: that opens or raises the window. Do not stop,
signal or restart the person's app or core. A window closing does not kill
its terminals.

## Helpers the person can see

When you start another coding agent as a helper, use
`hc run --bg --name <name> -- <agent command and args>`, then `hc wait <name>`
until it finishes, `hc read <name>` for its output, and
`hc send <name> <text>` to message it. Do not start agent CLIs directly from
your shell. Your core terminal is `$HC_TERMINAL_ID`; `canopy --skill` explains
how to work with the window. Canopy gives agents this short note through
per-session flags, without editing harness configuration.

Helpers started through hc are real child terminals, joined to their parent
on the canvas. The person can open them and send terminal input.
Session hooks report in-process helpers as draggable sub-agent cells, named
by their task description (agent type until it arrives). Click to follow the
read-only transcript in the side panel. Cells change from working to done;
a restart of the same id restores working. "Message through parent" pastes
`Message your sub-agent <name> (<id>): ` into the parent without Enter
(`canvas.sub_message {terminal, id}`). The parent's "Clear finished" menu or
`canvas.clear_finished {terminal}` clears done cells; SessionEnd clears all
its sub-agent labels. These cells describe work, not terminal processes.

## Agent flags and note in `programs.json`

The built-in table supplies a top-level `note` string. A person's
`programs.json` replaces it when `note` is set, including an empty string.
An agent entry's `flags` are extra argv words appended to its `run` command;
they merge like `run`. `{canopy}` expands to the shell-quoted absolute app
binary, `{note}` to the plain note, and `{note_json}` to a JSON string literal
(also a valid TOML basic string). JSON-shaped flag words are expanded as JSON
so embedded quotes and newlines stay valid.

Claude gets its per-session `--settings` hooks JSON followed by
`--append-system-prompt` and the note. Codex gets
`-c developer_instructions={note_json}`. This replaces any
`developer_instructions` in the person's Codex config for Canopy-started
Codex sessions only.

Local menu launches receive the flags directly. In plain local zsh and fish
terminals Canopy creates, typed agents receive the same flags through shell
functions generated from `run` and `flags` under the app home's `shell/`.
The function runs `command <run> <flags>` with the person's arguments. If
those arguments already contain the first flag word (for example
`--settings`), it passes them through without adding anything.
The startup file restores the inherited `ZDOTDIR` or `XDG_DATA_DIRS` before
children run; zsh sources the person's `.zshenv` and reads their remaining
startup files normally. Canopy never edits the person's shell files.
Remote terminals, bash, other shells and Windows have no typed-agent wrapper;
remote and Windows menu launches also omit these flags.


## Show something when asked

New spaces and tabs are shown immediately: only create them when asked.
Pick an explicit, useful name for every new space and tab; don't rely on
default names. Use IDs from `ls` rather than guessing or acting on the wrong
focused pane.

```sh
canopy do space.new --name api --folder /path/to/api
canopy do tab.new --name "fix login" --argv '["claude"]' --task "Look at the login bug"
canopy do pane.split --direction right --pane p12
canopy do agent.focus --terminal t7
```

For a remote space:

```sh
canopy do space.new --name "remote api" --machine vps --folder /srv/api
```

The folder is on that machine; tabs and splits in the space stay there.
New-space keys and clicks follow the focused space's machine. Script commands
without `--machine` still use this computer; `--machine ''` explicitly chooses it.
The agent menu checks and starts agents in the remote Unix user's
login/interactive shell (sh, bash, zsh, dash, ksh or fish), including its
configured PATH. Other shells keep the sh launcher.
`terminal.open --folder /path --place down` starts the split in that folder,
including when `--argv` supplies a program. Without `--folder`, it follows the
focused terminal's folder. If an inherited local folder was deleted, new spaces,
tabs and splits retry in that machine's home. An explicit invalid folder
still fails.
`hc machine ls` lists machines. Use `hc run --on vps ...`, `hc exec --on vps ...`
and `hc file ... --on vps ...` for remote work, not your local file tools.

`do` accepts `--key value` arguments (JSON for arrays/objects) or one JSON
object. Unknown argument names fail and list valid names. Target IDs (`id`,
`pane`, `space`, `target`, `terminal`) must be strings; other non-null types fail before
the current-target default can apply. Null keeps its omitted-argument meaning.
Use `commands` to confirm the command's exact shape:

```sh
canopy do pane.split '{"direction":"right","pane":"p12"}'
```

`pane.swap {pane?, direction | target}` trades the places of `pane` (default:
the focused one) and its neighbor in a direction, or `target`, another pane of
the same tab. Give `direction` or `target`, not both. Focus stays with the same pane:

```sh
canopy do pane.swap '{"pane":"p12","target":"p14"}'
```

`terminal.select_all {terminal?}` selects a terminal's whole screen and scrollback,
ready for `terminal.copy`. Without a target it follows keyboard focus. In vim and
other alternate-screen apps, it selects only that screen. Cmd+A runs it on macOS;
Linux and Windows have no default, so Ctrl+A stays with the shell.

`terminal.search` opens a find bar in the focused terminal. It searches that
terminal's screen and scrollback; Enter and Shift-Enter move between matches,
Escape closes the bar while its find field has focus, and each terminal keeps its own query (reopening selects it, so typing replaces it).
Clicking the terminal while the bar is open puts typing, including Escape, back in the program; the bar and its matches stay.

It prints the created IDs (for example `{ "pane": "...", "terminal": "..." }`)
or `{ "error": "..." }` and exits 1. Read the result, then inspect `ls` or
`status` to confirm. Commands invoked by an agent never open a missing-argument
dialog: supply all required arguments. A successful launch reply means the
terminal was created, not that its program started correctly. If it exits
with a nonzero code within two seconds, the window shows an error toast with
the exit code and last output lines.

A terminal started with hc opts into the window with `herdr.space=<id>`.
`herdr.tab=<id>` chooses its tab; without it, Canopy creates a tab in that space.
Unlabeled workers stay hidden. Do not add labels just to run a background
worker. Use app commands when the
user wants to see a new tab or split, hc for orchestration that need not change
the screen.

## Customize layouts, keys, or mods

First read `~/.config/canopy/AGENTS.md` (or `$HERDR_APP_HOME/AGENTS.md` if
set). It is the installed guide to settings, layouts, commands and mods.
Read the generated `.d.ts` files beside it for full API details; do not edit
those files or the generated guide. The app rewrites them on start.

- `terminal_line_height` (default 1, 0.8..2) adjusts terminal rows;
  `terminal_scroll_speed` (default 1, 0.1..5) adjusts wheel and trackpad
  scrolling, including mouse reports. Set them in settings.json or the
  layout's look; the layout wins.
- Home defaults to `~/.config/canopy`; `HERDR_APP_HOME` selects another.
- Mods live in `mods/<name>/` with `main.js` and optional `mod.json`.
  They are trusted JavaScript with command and network access.
  `"focusable": true` in `mod.json` lets `pane.go` move onto a copy; it receives
  `"focus"` from `nextCommand()` and focuses its own keyboard handle.
- A mod's `watchFolder(dir)` promise has `cancel()`; cancel it when the folder
  is no longer shown. It resolves with `[]` and releases only that call.
- Shipped mods belong to Canopy: duplicate with `mod.clone` before editing.
  Shipped layouts likewise use `layout.clone` for a personal copy.
  Discover required arguments with `canopy commands`.
- Save to reload. A broken save keeps the old version running and reports
  an error in `status`; a still-working screen is not proof the edit worked.
- Make only the requested change. Don't touch unrelated mods or settings.
- Use theme colors. Do not retain a mod's `cx` across callbacks; each
  callback receives its own. Native animations, not redraw timers, keep
  the window cheap to run. Root `on_size_change((size, cx) => …)` reports
  logical pixels after layout, including resizes without pointer movement.
  Optional `View.close(cx)` gets one bounded final call; start cleanup there,
  not in an earlier task. The installed guide explains its 250 ms quit budget
  and which cleanup commands go out before shutdown.

After every mod change:

```sh
canopy check ~/.config/canopy/mods/<name>
canopy status
canopy shot /tmp/mod.png   # then look at the picture
```

`check` loads the mod and renders it once. It exits nonzero with the exact
error. Its shared state starts empty and stays in memory; it leaves shared.json
untouched. Never call a mod done without a passing check. Then confirm it is
shown correctly in the running window with `shot`; don't claim a visual
check you haven't made.

`canopy shot [file.png]` saves the window's last frame and prints its
absolute path (default: `herdr-shot.png` in your current folder). It is the
app's own rendering at full pixel size: terminals, images, overlays and
floats included, even when the window is covered or minimized. No screen
capture or permissions. It needs a running window that has drawn a frame;
`HERDR_APP_HOME` selects the window, as for `status`.

## Problems and safety


Never experiment with pointer/key automation on the person's real display.
When developing the app on Linux, its `scripts/sandbox` provides an isolated
home, core and display: `up`, `do`, `status`, `shot`, then `down` when done.
Never stop a server or session you didn't start.

Canopy main layout is a canvas. `canvas.zoom_in`, `canvas.zoom_out`,
`canvas.zoom_reset`, `canvas.fit` control its camera. `panel.close` hides the
terminal panel without ending the terminal. Geometry is in app-home canvas.json;
`canvas_color` in settings.json overrides the theme background (null resets).
`panel.open {terminal}` selects the terminal shown in the canvas panel. The
`classic` layout preserves the previous sidebar and tabs. Canvas zoom bindings
apply to the `main` layout; other layouts retain app zoom.

Canvas navigation: scroll pans; pinch or Cmd+scroll (Ctrl on Linux) zooms at the cursor.
Hold Space with canvas focus and left-drag anywhere to pan, with hand cursors.
`canvas.arrange {layout?: "tidy" | "row" | "column" | "wires", space?, picker?}`
arranges selected cells or the current space; no layout repeats the last choice.
`canvas.undo` (Cmd+Z / Ctrl+Shift+U while the canvas has focus) restores
geometry before the last move, resize or arrange. Cells keep a 24 px gap;
spaces grow to hold them and push other spaces.

Empty-canvas left-drag and middle-mouse drag also pan; an empty click focuses the canvas.
Focused terminals still receive Space. Right-click keeps the canvas menus.
Canvas space labels drag the space and its cards; bottom-right handles resize with card padding.
Space label menus offer Rename, Color, Show frame and confirmed Delete space.
Empty-canvas menus create terminals in the space folder or home, installed agents,
and root spaces through the Choose space dialog (previously used space folders, or New space: name, optional folder, recent folders). `canvas.create` takes an optional world
`point` (default: the middle of the view), optional `space` (default: the space under the point, else the root), `argv`, or `kind: "space"`; `canvas.space_style` takes
`id` and optional palette `color` (without color it toggles the frame).
`view.agents`, `view.all`, `view.terminals` select the saved canvas view, bound to
Cmd+Shift+1/2/3 on macOS and Ctrl+Shift+1/2/3 elsewhere. Hidden cards lose panel focus.
`panel.pop_out` takes an optional `terminal` (defaults to the panel); `panel.bring_back`
takes `terminal`. Pop-outs last only for the current run and keep the terminal alive.

Canvas file previews: `preview.open {path?, point?, place?}` opens an absolute local
path in a read-only panel tab by default. Reopening activates the tab. Omit path for
the native picker, which places a canvas card. `place: "canvas"` or an explicit
world `point` also places a card. Open file... and Finder drops keep placing cards.
Clicking a pill opens or activates its terminal tab. Closing a tab keeps its terminal
running; the panel close button closes all panel tabs. Terminal pop-out is unchanged.
The header shows a breadcrumb and a folder button toggles the right file tree,
saved as `panel.tree_hidden` in canvas.json. The tree shows the folder of the space last
clicked on the canvas (`panel.space`); clicking pills does not change it. File tabs have a pin
button to place a canvas card. The Files mod supplies the bounded lazy tree and
highlighted code editor. Markdown uses the inert document renderer when read-only.
File panel headers have an editor split button: the main button repeats the last choice,
and its menu offers Open here and installed external editors (local files only).
The choice is saved as `panel.editor` in canvas.json; initially it uses the first installed
editor, or Open here. Editors come from programs.json `open` argv and macOS `app`.
Open here edits in place, including markdown source, with highlighting. Save or cmd+s
saves and keeps editing; Done saves pending edits and returns to the read-only view.
Unsaved marks a draft. Switching tabs or closing the tab drops it without a dialog.
Saves use `writeTextFile(path, text, expected)`; conflicts show inline and keep the draft.
Canvas cards stay read-only with their existing viewer, drag header and resize grip;
there is no card-header Edit button.
Files follow folder watcher changes, show File deleted when missing, and never
execute content or load remote images.

Preview quotes: finishing a selection in a preview opens the quote popover, also opened by `quote.menu {path, start?, end?, text, point?}` (point is window pixels, default the pointer). `quote.send {path, start?, end?, text, message?, terminal | argv}` opens an agent in the canvas panel and pastes the quote, then a blank line and the trimmed message when given, without Enter. New agents start in the panel space. Delivery waits up to 30 seconds for bracketed paste; on timeout nothing is sent. Paths must be absolute; source lines are 1-based and inclusive. Delivery uses a path relative to the agent cwd when possible.

Canvas space titles rename inline on double-click or Rename. Person-invoked local
`space.new` without a folder opens New space; explicit command arguments remain supported.
`canvas.create {kind: "space", name, folder, point?}` creates directly when `folder`
is present (an empty string means no folder); omitting it opens the Choose space dialog.

## Canvas selection and views

Click a cell to select it. The canvas keeps the keyboard until the terminal
panel is clicked. `canvas.close` (Backspace or Delete) closes the selection: running agents
always ask "Terminate <name> and close?"; plain terminals ask "Close this
terminal?" with "Don't ask again". That choice writes
`confirm_close_terminal: false` to settings.json (default true). Done
sub-agent cells clear; working ones refuse with a toast.

Views are in the macOS View menu or the Linux titlebar's primary menu
(`app.menu`, optionally with a `group`);
the active view is checked. `view.agents`, `view.all`, `view.terminals` and
their keys still work. The canvas toolbar keeps the file button.
