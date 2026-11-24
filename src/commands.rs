// Modified for Canopy.
//! Every command the app runs, defined once. Keys, menus, mods and
//! `canopy do` all run these; key checks, `canopy commands` and the docs come
//! from this list. Mods' own commands are `<mod>.<command>`.

use serde_json::{Value, json};

pub struct Arg {
    pub name: &'static str,
    pub required: bool,
}

pub struct Command {
    pub name: &'static str,
    pub args: &'static [Arg],
    pub about: &'static str,
}

const fn req(name: &'static str) -> Arg {
    Arg { name, required: true }
}

const fn opt(name: &'static str) -> Arg {
    Arg { name, required: false }
}

const fn cmd(name: &'static str, args: &'static [Arg], about: &'static str) -> Command {
    Command { name, args, about }
}

pub const COMMANDS: &[Command] = &[
    cmd("canvas.zoom_in", &[], "Zoom the canvas in one step."),
    cmd("canvas.zoom_out", &[], "Zoom the canvas out one step."),
    cmd("canvas.zoom_reset", &[], "Reset canvas zoom."),
    cmd("canvas.arrange", &[opt("layout"), opt("picker"), opt("space")], "Arrange selected cells or the current space: tidy, row, column or wires. Omit layout to repeat the last; picker opens the layout menu."),
    cmd("canvas.undo", &[], "Restore geometry before the last canvas move, resize or arrange."),
    cmd("canvas.tile", &[], "Tile view: fit every expanded cell of the current space into the visible canvas, tmux-style, with the selected one largest; collapsed cells line up below. Repeat to put everything back."),
    cmd("canvas.fit", &[], "Fit the selection, else everything in the current space or Home; repeat to restore the previous camera."),
    cmd("canvas.mode", &[], "Enter canvas mode: its keys take priority and unbound keys are swallowed until Enter or Escape."),
    cmd("canvas.leave", &[], "Cancel a keyboard connection, or leave canvas mode with selection unchanged."),
    cmd("canvas.enter", &[], "Complete a keyboard connection, or expand the selected cell and leave canvas mode."),
    cmd("canvas.go", &[req("direction")], "Select the nearest shown cell left, right, up or down without expanding it."),
    cmd("canvas.space", &[req("direction")], "Go into the next or prev space, space N (1-based, top strip order) or a space id, or the recent / oldest one entered; its cells fill the canvas."),
    cmd("canvas.home", &[], "Leave the space for Home: space cards and cells outside any space."),
    cmd("canvas.select_all", &[], "Select every terminal and agent cell in the current place; only with canvas focus, so an entered terminal keeps its own select-all."),
    cmd("canvas.fit_all", &[], "Fit everything in the current place, whatever is selected; repeat to restore the previous camera."),
    cmd("canvas.collapse", &[], "Collapse the selected terminal cell."),
    cmd("canvas.connect", &[], "Start a keyboard connection from the selected cell; move to the other cell and press Enter."),
    cmd("canvas.nudge", &[req("direction")], "Move the selected cell left, right, up or down by one canvas gap, with one undo step."),
    cmd("canvas.new", &[opt("like")], "Create a terminal beside the selected cell, in its space. With like: true, start what the selected cell runs (an agent or other startable program), else a plain terminal."),
    cmd("canvas.agents", &[], "Open the installed agent list beside the selected cell."),
    cmd("quote.menu", &[req("path"), opt("start"), opt("end"), req("text"), opt("point"), opt("pane"), opt("kind"), opt("source")], "Open the quote popover at point or the pointer: agents wired to the quoted pane first, then other running agents, then installed agents to start with the quote."),
    cmd("canvas.close", &[opt("terminal"), opt("terminals"), opt("confirmed"), opt("dont_ask"), opt("pane"), opt("panes")], "Close every selected pane (else the focused one) behind one dialog listing them, the named terminal, or one file or mod pane by id (pane; a mod pane holding text refuses); terminals and panes are the confirmed lists, closed without asking; working sub-agents refuse. Dialogs pass confirmed as agent or terminal, and dont_ask disables future plain-terminal prompts."),
    cmd("canvas.pane_new", &[req("kind"), opt("path"), opt("url"), opt("udid"), opt("wire"), opt("point"), opt("place"), opt("pinned")], "Open a file, browser, sim (on a Mac with Xcode), or mod:<name> pane. Files require path. A browser accepts an HTTP(S) address, an absolute local path or a file URL, and opens New tab when url is omitted. A sim opens udid (a simulator or \"booted\") or lists devices to pick. place is \"panel\" (a tab in the Files panel, the default for browsers and sims), \"canvas\" (at point, the middle of the view by default) or \"pinned\" (the top right of the screen; pinned: true is the same). Pass wire to place it beside a terminal and wire it in that terminal's space. A file path already open moves its pane. The reply includes pane, kind, wired and the pane's path, url or udid when applicable."),
    cmd("canvas.pane_dock", &[opt("pane"), opt("docked")], "Dock a browser or sim pane as a tab in the Files panel, or with docked false move it back onto the canvas at its saved place. Without docked it toggles. Acts on pane, else the selected pane. Replies with pane and docked."),
    cmd("canvas.pane_pin", &[opt("pane"), opt("pinned")], "Pin a pane where it shows on screen, so it stays there while the canvas pans and zooms, or with pinned false move it back onto the canvas. Without pinned it toggles. Acts on pane, else the selected pane. Replies with pane and pinned."),
    cmd("canvas.pane_set", &[req("pane"), req("props")], "Merge props (an object) into a mod pane's saved props; mods save their state with it, and a string props.text is what canopy pane read returns. Replies with nothing."),
    cmd("canvas.hand", &[opt("on")], "Hand tool: while on, a left drag anywhere on the canvas pans, as with Space held. Without on it toggles."),
    cmd("canvas.pane_read", &[req("pane")], "A pane's text inside a <pane-content> envelope that marks it as untrusted, cut at 256 KiB. Replies with text."),
    cmd("canvas.pane_shot", &[req("pane"), opt("path")], "Save a pane as drawn on screen to a PNG (default: <temp>/canopy/<pane>.png); refuses when it is off screen. Replies with path."),
    cmd("canvas.pane_open", &[req("pane"), req("target"), opt("tab")], "Point a browser or simulator pane at a target. A browser loads an HTTP(S) address, an absolute local path or a file URL in its active tab and makes it Home. Pass tab: new to open the URL in a new browser tab. A bare localhost:3000 uses http. Open URL without a target focuses the address field when a browser pane is given or selected. A sim takes a udid or \"booted\" and refreshes the screenshot when already showing that device."),
    cmd("canvas.browser_tab_new", &[opt("pane"), opt("url")], "Open a new tab in a browser pane. Pass an HTTP(S) address, an absolute local path or a file URL, or omit url for an empty tab. Replies with pane and tab. With no pane and no single browser pane selected, runs canvas.new."),
    cmd("canvas.browser_tab_close", &[opt("pane"), opt("tab")], "Close a tab in a browser pane. Without tab, closes the active tab. Closing the last tab leaves an empty tab. With no pane and no single browser pane selected, runs tab.close if bound."),
    cmd("canvas.browser_tab_select", &[opt("pane"), req("tab")], "Switch to a browser tab by id. Pass pane or use the selected browser pane."),
    cmd("canvas.browser_back", &[opt("pane")], "Go back in a browser pane's history (pane, else the selected pane)."),
    cmd("canvas.browser_forward", &[opt("pane")], "Go forward in a browser pane's history (pane, else the selected pane)."),
    cmd("canvas.browser_reload", &[opt("pane")], "Reload a browser pane's page (pane, else the selected pane)."),
    cmd("canvas.browser_home", &[opt("pane")], "Load the URL a browser pane was last opened with (pane, else the selected pane)."),
    cmd("canvas.browser_pick", &[opt("pane")], "Start annotating a browser page with the Select tool: click an element to pin a comment on it (pane, else the selected pane)."),
    cmd("canvas.browser_annotate", &[opt("pane")], "Start or stop annotating a browser page: select elements, draw, and send comments to an agent. Acts on pane, else the selected browser pane."),
    cmd("canvas.browser_shot", &[opt("pane")], "Save a browser pane's page to <temp>/canopy/<pane>.png and toast the path (pane, else the selected pane)."),
    cmd("canvas.browser_external", &[opt("pane"), opt("tab")], "Open a browser pane's page, or the tab named by tab, in the system browser (pane, else the selected pane)."),
    cmd("canvas.browser_copy", &[opt("pane")], "Copy a browser pane's page URL to the clipboard and toast it (pane, else the selected pane)."),
    cmd("canvas.browser_viewport", &[req("size"), opt("pane")], "Set a browser viewport to responsive, 1280x800, 1024x768, mobile (375x812) or tablet (768x1024). The desktop alias selects 1280x800. Fixed CSS sizes scale to fit and center inside the pane. Responsive pages reflow as the pane resizes. Pass pane or use the selected browser pane."),
    cmd("canvas.browser_expand", &[opt("pane")], "Expand a browser pane's live page into a tab of the top strip that fills the canvas area, or collapse it again (pane, else the selected pane). Escape, any space tab or Home, and closing the pane collapse it."),
    cmd("canvas.browser_pop", &[opt("pane")], "Pop a browser pane out into its own window with the same header, every tab moving with it, or bring it back (pane, else the selected pane). The pane shows \"Popped out\" meanwhile."),
    cmd("canvas.browser_cookies", &[req("keep")], "Set how long browser panes keep cookies and site data: until_quit or forever."),
    cmd("canvas.browser_links", &[req("open")], "Set where HTTP and HTTPS links clicked in terminals open: built_in (a browser tab) or default (the system browser)."),
    cmd("canvas.browser_allow_folder", &[opt("pane")], "Ask the person at the window to pick a directory containing the open local file. WebKit can then load assets from that folder for this tab."),
    cmd("canvas.browser_open_file", &[opt("pane")], "Pick an .html or .htm file and open it in a new tab of a browser pane (pane, else the selected pane). Opens a native picker for the person at the window."),
    cmd("canvas.browser_clear", &[opt("pane"), opt("confirmed")], "Delete the cookies, storage and cache of every browser pane (asks first unless confirmed)."),
    cmd("canvas.browser_inspect", &[opt("pane")], "Open the Web Inspector for a browser pane in its own window, after snapping the pane to a usable size at 100% zoom."),
    cmd("canvas.sim_refresh", &[opt("pane")], "Take a new screenshot for the simulator pane and try live again after a failure. Acts on pane, else the selected simulator pane."),
    cmd("canvas.sim_device", &[opt("pane")], "Pick another simulator for a simulator pane from a menu (Xcode's version on top); picking boots it. Acts on pane, else the selected simulator pane."),
    cmd("canvas.sim_hub", &[], "Open Xcode's Device Hub to use a simulator."),
    cmd("canvas.sim_home", &[opt("pane")], "Press Home on a simulator pane's live device. Acts on pane, else the selected simulator pane."),
    cmd("canvas.sim_screenshot", &[opt("pane")], "Save a simulator pane's live screen to the Desktop as a PNG. Replies with path. Acts on pane, else the selected simulator pane."),
    cmd("canvas.sim_record", &[opt("pane")], "Start recording a simulator pane's screen to a movie on the Desktop, or stop and save it. Replies with the path when it starts. Acts on pane, else the selected simulator pane."),
    cmd("canvas.sim_annotate", &[opt("pane")], "Start or stop annotating a still of a simulator screen. Acts on pane, else the selected simulator pane."),
    cmd("canvas.annotate_send", &[opt("pane")], "Submit the annotated screenshot and comments to the sole wired agent, or the one chosen with canvas.annotate_agent. Acts on pane, else the selected annotation."),
    cmd("canvas.annotate_action", &[req("pane"), req("action")], "Run a tool, colour, stroke, undo or redo in an active annotation, or choose its target agent with `agent:<terminal>`. Used by its menus."),
    cmd("canvas.annotate_agent", &[opt("pane")], "Open the menu that chooses which agent an annotation is sent to (wired agents first). Acts on pane, else the selected annotation."),
    cmd("canvas.sim_zoom", &[opt("step"), opt("pane")], "Resize a simulator pane: step actual (one device point per canvas unit, requires a live frame), default (340x700 on screen), in or out. Omit step for actual. Acts on pane, else the selected simulator pane."),
    cmd("canvas.sim_shutdown", &[opt("pane"), opt("close")], "Shut down the device in the named or selected simulator pane. On success, show the device picker or close the pane if close is true. The result appears in a toast."),
    cmd("canvas.sim_pop", &[opt("pane")], "Pop a simulator pane's device out into its own window, or bring it back. Acts on pane, else the selected simulator pane."),
    cmd("canvas.sim_reveal", &[req("path")], "Show a saved screenshot or recording in Finder."),
    cmd("canvas.sim_input", &[req("pane"), req("action"), opt("x"), opt("y"), opt("x1"), opt("y1"), opt("x2"), opt("y2"), opt("ms"), opt("text"), opt("button")], "An agent's input for a live simulator pane, in device points from the top left: action tap (x, y), swipe (x1, y1, x2, y2, ms, default 300), type (text, US keyboard) or press (button: home, lock, power, siri, volume-up, volume-down, action). Starts the device if the pane is off screen."),
    cmd("canvas.sync", &[], "Sync input (like tmux synchronize-panes) for the selected panes, or turn it off: typing in one goes to all. Esc or deselecting ends it."),
    cmd("canvas.equalize", &[], "Give the selected panes (else the focused pane) the standard size (settings terminal_size); tile view re-tiles."),
    cmd("canvas.front", &[opt("terminal")], "Bring the selected panes (else the focused pane, or a named terminal outside the selection) in front of every other pane; the order is saved."),
    cmd("canvas.back", &[opt("terminal")], "Send the selected panes (else the focused pane, or a named terminal outside the selection) behind every other pane; the order is saved."),
    cmd("canvas.clear_finished", &[req("terminal"), opt("id")], "Clear finished sub-agent cells and their labels, optionally one id."),
    cmd("wire.connect", &[req("a"), req("b")], "Draw a Wire edge between two panes of one space (pane ids: a terminal address, sub:<parent>:<id>, or p<N> for a file pane). Panes in different spaces, and two file panes, are refused."),
    cmd("wire.remove", &[req("a"), req("b")], "Remove a drawn Wire edge between two cells."),
    cmd("quote.send", &[req("path"), opt("start"), opt("end"), req("text"), opt("message"), opt("terminal"), opt("argv"), opt("pane"), opt("kind"), opt("page"), opt("region"), opt("crop"), opt("source"), opt("images")], "Paste a quote and optional message, waiting up to 30 seconds for bracketed paste. Annotations also submit the prompt."),
    cmd("canvas.expand", &[opt("terminal")], "Expand the named or last selected terminal cell into a live terminal in place, or collapse it."),
    cmd("panel.open", &[req("terminal")], "Expand a terminal's cell in the canvas, as canvas.expand does (it no longer opens the side panel)."),
    cmd("panel.close", &[], "Close the canvas file panel."),
    cmd("view.agents", &[], "Show agent cards."),
    cmd("view.all", &[], "Show all cards."),
    cmd("view.terminals", &[], "Show terminal cards."),
    cmd("canvas.space_style", &[req("id"), req("color")], "Set a space color."),
    cmd("canvas.pane_color", &[req("pane"), opt("color")], "Set a canvas pane's hue. Pass null or omit color to inherit the space's hue."),
    cmd("panel.editor", &[opt("choice"), opt("done"), opt("path"), opt("dirty")], "Choose a file panel editor, return to the read-only view after saving, or report the current editing file's dirty state."),
    cmd("preview.open", &[opt("path"), opt("point"), opt("place")], "Open a file tab, or choose a file; place canvas or point pins a canvas card."),
    cmd("canvas.create", &[opt("point"), opt("kind"), opt("space"), opt("argv"), opt("name"), opt("folder")], "Create a canvas terminal or space at a world point (default: the middle of the view); a space with a folder (empty: the root) is made directly, without one the Choose space dialog opens."),
    cmd("panel.pop_out", &[opt("terminal")], "Pop a terminal cell into its own window (default: the last selected cell)."),
    cmd("panel.bring_back", &[req("terminal")], "Bring a popped terminal back."),
    // What exists.
    cmd("space.new", &[opt("name"), opt("folder"), opt("tab"), opt("argv"), opt("task"), opt("machine")],
        "Make a space: its name (default: none, so it's called after where its first terminal is), the folder its terminals start in (default: the focused terminal's), its first tab's name (default: none, so it's called by its number), optionally an agent (argv) and its first message (task), and the machine it lives on (`hc machine ls`; default the focused space's machine for keys and clicks, this computer for scripts; empty string: this computer; its terminals start in that machine's home unless folder)."),
    cmd("space.close", &[opt("id"), opt("confirmed")], "Close a space and end its terminals (asks first unless confirmed)."),
    cmd("space.rename", &[opt("id"), opt("name")], "Rename a space (asks the person when no name; an empty name: called after where its first terminal is again)."),
    cmd("space.move", &[req("id"), req("index")], "Move a space to a position (1 = first)."),
    cmd("tab.new", &[opt("name"), opt("space"), opt("folder"), opt("argv"), opt("task"), opt("mod"), opt("props")],
        "Make a tab in a space (default: the selected one; name default: none, so it's called by its number) with a terminal in folder (default: the focused terminal's), optionally an agent (argv) and its first message (task); or with a mod as its only pane (`mod`, its `props`: a file, a board). With no space open, a person's new tab starts a space (as space.new)."),
    cmd("tab.close", &[opt("id"), opt("confirmed")], "Close a tab and end its terminals."),
    cmd("tab.rename", &[opt("id"), opt("name")], "Rename a tab (asks the person when no name; an empty name: called by its number again)."),
    cmd("tab.move", &[req("id"), req("index")], "Move a tab to a position in its space (1 = first)."),
    cmd("pane.split", &[req("direction"), opt("pane"), opt("argv"), opt("mod"), opt("props")], "Split a pane (default: the focused one) \"right\" or \"down\" with a new terminal, or with a mod (`mod`, its `props`) beside it."),
    cmd("pane.close", &[opt("pane")], "Close a pane and end its terminal."),
    cmd("terminal.open", &[opt("argv"), opt("folder"), opt("place"), opt("name")], "Start a terminal: as a new tab (place \"tab\", default), or \"right\"/\"down\" of the focused pane."),
    cmd("terminal.keep", &[req("terminal"), opt("place")], "Give a running terminal without a pane (a sub-agent) a pane: a new tab, or \"right\"/\"down\"."),
    cmd("terminal.park", &[req("terminal")], "Take a terminal's pane away and disable core keep; it keeps running (a sub-agent shows under its parent again)."),
    cmd("terminal.pin", &[req("terminal"), req("pinned")], "Pin an agent to the top of the agent lists (pinned true) or unpin it (false). Pins are kept with the terminal, so every Canopy client shows them, and end with it."),
    cmd("terminal.kill", &[req("terminal")], "End a terminal and its sub-agents."),
    // What you see.
    cmd("space.select", &[opt("index"), opt("id")], "Show a space (by position or id)."),
    cmd("space.next", &[], "Show the next space."),
    cmd("space.prev", &[], "Show the previous space."),
    cmd("tab.select", &[opt("index"), opt("id")], "Show a tab (by position in the space, or id)."),
    cmd("tab.settle", &[opt("id")], "Put a tab away on this screen (default: the shown one): out of the lists, still running. It comes back when shown or when an agent in it needs you."),
    cmd("tab.unsettle", &[opt("id")], "Bring a settled tab back to the lists."),
    cmd("tab.next", &[], "Show the next tab."),
    cmd("tab.prev", &[], "Show the previous tab."),
    cmd("pane.zoom", &[opt("pane")], "Show only this pane of its tab, or all again."),
    cmd("pane.focus", &[opt("pane"), opt("terminal")], "Focus a pane (or the pane showing a terminal); a file, browser or other canvas pane is selected and brought into view."),
    cmd("pane.go", &[req("direction")], "Focus the pane or focusable mod to the \"left\", \"right\", \"up\" or \"down\"."),
    cmd("pane.swap", &[opt("direction"), opt("pane"), opt("target")], "Swap a pane (default: the focused one) with its neighbor in a direction, or with another pane of its tab (target). Give exactly one of direction or target."),
    cmd("terminal.peek", &[req("terminal")], "Peek at a running terminal over the tab, with keep-as-tab and close controls."),
    cmd("terminal.show", &[opt("terminal"), opt("pane")], "Show a terminal in a pane for now, in place of its own (no terminal: back)."),
    cmd("terminal.select_all", &[opt("terminal")], "Select a terminal's whole screen and scrollback (default: the focused one), ready to copy. On the alternate screen, select only that screen."),
    cmd("terminal.copy", &[], "Copy the focused terminal's selection. With nothing selected, that key goes to the program."),
    cmd("terminal.scroll", &[opt("lines"), opt("pages"), opt("to"), opt("terminal")], "Scroll a terminal's view of its history (default: the focused one). Give exactly one of: lines (a whole number; negative goes up, toward history), pages (a number, fractions allowed; one page is the terminal's rows), to: \"top\" (oldest history) or \"bottom\" (the live screen). This scrolls Canopy's view, not the program."),
    cmd("terminal.paste", &[], "Paste into the focused terminal (copied files as if dropped; an image alone as terminal.paste_image pastes it)."),
    cmd("terminal.paste_image", &[], "Ctrl+V: the clipboard's image as a file on the focused terminal's machine, when that's another one; on Windows, paste clipboard text; otherwise Ctrl+V goes to the terminal."),
    cmd("terminal.search", &[], "Search the focused terminal's screen and scrollback; the find bar and query stay with that terminal."),
    cmd("agent.focus", &[req("terminal")], "Show an agent (sub-agents too). On the canvas it also enters its pane."),
    cmd("agent.next", &[], "Show the next agent, in the order the agents list shows them."),
    cmd("agent.prev", &[], "Show the previous agent."),
    cmd("layout.switch", &[opt("name")], "Show a layout (layouts/<name>.json), or the next one."),
    cmd("layout.clone", &[req("name"), opt("as")], "Duplicate a layout as <name>-copy (or as), then show it."),
    cmd("mod.clone", &[req("name"), opt("as")], "Duplicate a mod as <name>-copy (or as)."),
    cmd("layout.set", &[req("layout")], "Replace a personal layout tree; shipped layouts accept only click/look fields (not structure)."),
    cmd("layout.toggle", &[req("id")], "Hide or show a layout node."),
    cmd("layout.show", &[req("id")], "Show a layout node."),
    cmd("layout.hide", &[req("id")], "Hide a layout node."),
    cmd("layout.resize", &[req("id"), opt("px"), opt("weight")], "Size a layout node."),
    cmd("layout.select", &[req("stack"), req("child")], "Show one child of a stack."),
    cmd("layout.reset", &[opt("name")], "Reset a shipped layout and its clicks (the old file is kept aside)."),
    cmd("mod.open", &[req("name")], "Toggle a mod as the overlay its mod.json describes; calling again closes it."),
    cmd("overlay.close", &[opt("id")], "Close an overlay (default: the top dialog or popup)."),
    // The app.
    cmd("app.help", &[], "Toggle the keys dialog."),
    cmd("app.welcome", &[], "Show the welcome again."),
    cmd("app.palette", &[], "Toggle the command palette: every command by name, with its key; calling again closes it."),
    cmd("app.settings", &[opt("section")], "Open the settings (section: look, keyboard, shortcuts, machines, alerts)."),
    cmd("app.reset", &[], "The escape hatch: set the shown layout and keys aside, back to the default screen."),
    cmd("app.theme", &[req("name")], "Use a theme (a preset or themes/<name>.json)."),
    cmd("app.zoom_in", &[], "Make everything bigger."),
    cmd("app.zoom_out", &[], "Make everything smaller."),
    cmd("app.zoom_reset", &[], "Back to normal size."),
    cmd("app.menu", &[opt("group")], "Open the shared primary command menu, optionally a named group."),
    cmd("app.quit", &[], "Quit (macOS)."),
    cmd("app.hide", &[], "Hide the app (macOS)."),
    cmd("app.hide_others", &[], "Hide other apps (macOS)."),
    cmd("app.show_all", &[], "Show all apps (macOS)."),
    cmd("window.close", &[], "Close the window (on Linux, quits)."),
    cmd("window.minimize", &[], "Minimize the window."),
    cmd("window.maximize", &[], "Maximize the window, or back."),
    cmd("canvas.pane_quote", &[], "Open the quote menu for the last selected image pane."),
    cmd("canvas.pdf_find", &[], "Toggle text search in the last selected PDF pane."),
    cmd("canvas.pdf_highlight", &[], "Toggle text highlight mode in the last selected PDF pane."),
    cmd("canvas.pdf_prev", &[], "Go to the previous page in the last selected PDF pane."),
    cmd("canvas.pdf_next", &[], "Go to the next page in the last selected PDF pane."),
    cmd("canvas.pdf_clear", &[], "Clear all highlights in the last selected PDF pane."),
];

pub fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.name == name)
}

/// A key's `name:value` shorthand: the value fills the first argument (a
/// number fills `index` when there is one).
pub fn shorthand(c: &Command, value: &str) -> Value {
    let index = value.parse::<u64>().ok().filter(|_| c.args.iter().any(|a| a.name == "index"));
    match (index, c.args.first()) {
        (Some(i), _) => json!({ "index": i }),
        (None, Some(a)) => json!({ a.name: value }),
        (None, None) => Value::Null,
    }
}

/// Reject misspelled arguments before a command can use its current-target default.
/// Null is the argument-free form used by keys and mods.
pub fn validate_args(c: &Command, args: &Value) -> Result<(), String> {
    let valid = if c.args.is_empty() { "none".into() } else { c.args.iter().map(|a| a.name).collect::<Vec<_>>().join(", ") };
    match args {
        Value::Null => Ok(()),
        Value::Object(args) => {
            let unknown: Vec<_> = args.keys().filter(|key| !c.args.iter().any(|a| a.name == key.as_str())).map(String::as_str).collect();
            if !unknown.is_empty() {
                return Err(format!("{}: unknown argument(s): {}; valid arguments: {valid}", c.name, unknown.join(", ")));
            }
            for (key, value) in args {
                if matches!(key.as_str(), "id" | "pane" | "space" | "terminal" | "target") && !value.is_string() && !value.is_null() {
                    return Err(format!("{}: argument {key} must be a string (or null to omit it)", c.name));
                }
            }
            Ok(())
        }
        _ => Err(format!("{}: arguments must be an object; valid arguments: {valid}", c.name)),
    }
}

/// The first required argument that's missing (an empty string counts as missing).
pub fn missing(c: &Command, args: &Value) -> Option<&'static str> {
    c.args.iter().filter(|a| a.required).map(|a| a.name).find(|k| match args.get(k) {
        None | Some(Value::Null) => true,
        Some(Value::String(s)) => s.trim().is_empty(),
        _ => false,
    })
}

/// The list as data, for `canopy commands` and mods.
pub fn listing() -> Value {
    Value::Array(
        COMMANDS
            .iter()
            .map(|c| {
                let args: Vec<Value> = c.args.iter().map(|a| json!({ "name": a.name, "required": a.required })).collect();
                json!({ "name": c.name, "args": args, "about": c.about })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The guide agents read (written to the home folder) names every command.
    #[test]
    fn the_guide_names_every_command() {
        let guide = include_str!("../defaults/AGENTS.md");
        let missing: Vec<&str> = COMMANDS.iter().map(|c| c.name).filter(|n| !guide.contains(&format!("`{n}"))).collect();
        assert!(missing.is_empty(), "defaults/AGENTS.md (## Commands) doesn't name: {missing:?}");
    }

    #[test]
    fn unknown_arguments_are_rejected_for_every_command() {
        for c in COMMANDS {
            let error = validate_args(c, &json!({ "typo": "target" })).unwrap_err();
            assert!(error.contains(c.name), "{error}");
            assert!(error.contains("unknown argument(s): typo"), "{error}");
            assert!(error.contains("valid arguments:"), "{error}");
            assert!(validate_args(c, &Value::Null).is_ok());
            assert!(validate_args(c, &json!({})).is_ok());
            assert!(validate_args(c, &json!(["target"])).is_err());
            let args: serde_json::Map<String, Value> = c.args.iter().map(|a| (a.name.into(), json!("target"))).collect();
            assert!(validate_args(c, &Value::Object(args)).is_ok());
        }
        let close = find("space.close").unwrap();
        assert_eq!(validate_args(close, &json!({"space": "X", "confirmed": true})),
            Err("space.close: unknown argument(s): space; valid arguments: id, confirmed".into()));
        assert!(validate_args(close, &json!({"id": "X", "confirmed": true})).is_ok());
        assert!(validate_args(find("panel.editor").unwrap(), &json!({"path":"/tmp/file.rs", "dirty":true})).is_ok());
        // The space dialog's call; a missing `folder` once made every Create a silent no-op.
        let create = find("canvas.create").unwrap();
        assert!(validate_args(create, &json!({"kind": "space", "point": [1, 2], "name": "", "folder": "/tmp"})).is_ok());
    }

    #[test]
    fn malformed_targets_cannot_fall_back_to_the_current_work() {
        for (name, arg) in [("tab.close", "id"), ("pane.close", "pane"), ("space.close", "id"), ("tab.new", "space"), ("terminal.show", "terminal"), ("pane.swap", "pane"), ("pane.swap", "target")] {
            let c = find(name).unwrap();
            for value in [json!(123), json!(false), json!([]), json!({"id": "target"})] {
                let error = validate_args(c, &json!({arg: value})).unwrap_err();
                assert!(error.contains(name) && error.contains(arg), "{error}");
            }
            // Null keeps its existing omitted-argument meaning for optional targets.
            for value in [Value::Null, json!("target")] {
                assert!(validate_args(c, &json!({arg: value})).is_ok());
            }
        }
    }

    #[test]
    fn names_are_unique_and_shorthand_fills_the_first_argument() {
        let mut names: Vec<&str> = COMMANDS.iter().map(|c| c.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), COMMANDS.len());
        assert_eq!(shorthand(find("tab.select").unwrap(), "3"), json!({ "index": 3 }));
        assert_eq!(shorthand(find("pane.split").unwrap(), "right"), json!({ "direction": "right" }));
        let move_ = find("space.move").unwrap();
        assert_eq!(missing(move_, &json!({ "id": " ", "index": 1 })), Some("id"));
        assert_eq!(missing(move_, &json!({ "id": "sp1", "index": 1 })), None);
        // Names are optional: none is Canopy's automatic one.
        assert_eq!(missing(find("space.new").unwrap(), &json!({})), None);
    }
}
