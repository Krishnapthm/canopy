# Troubleshooting

Start here when something is not working. Problems are grouped by what you see. Each feature page also has its own troubleshooting section for problems specific to it, linked at the end.

## Canopy does not start

- **Canopy says it needs herdr-core, or a newer one.** See [herdr-core](herdr-core#troubleshooting).
- **Canopy cannot connect to the core.** Fix the error it shows, then click **Retry**.
- **Nothing opens at all.** Check the [install instructions](https://github.com/Krishnapthm/canopy#install), then run `canopy` from a terminal to see any error it prints.
- **The window is blank after waking a Mac.** Wake the display. Canopy draws again when the window is visible.

## Typing and scrolling

- **Typing into a terminal does nothing.** The pane is selected, not entered. Double-click it, or select it and press `Enter`.
- **Single keys move or close panes instead of typing.** The canvas has the keyboard (canvas mode). Double-click the terminal you want to type in.
- **Scrolling moves the canvas instead of the output.** Enter the terminal first. Scrolling over a terminal you have not entered pans the canvas.
- **`Esc` does not leave a terminal.** That is on purpose: `Esc` belongs to the program. Press `Cmd+Esc` (`Ctrl+Esc` on Linux and Windows), or click empty canvas.

## Agents

- **An agent CLI is missing from the Agent menu.** Canopy only lists CLIs it can find on your `PATH`. See [Agents](Agents#troubleshooting) for `programs.json` and aliases.
- **An agent does not know about panes, wires or helpers.** It has not learned Canopy. See [Agent Setup](Agent-Setup).
- **An agent says it has no wires.** The wire must join the two panes in the same space. See [Pair Agents](Pair-Agents#troubleshooting).
- **I did not get a sound or toast when an agent needed me.** Check that sounds are on in [Settings](Settings-and-Themes). Toasts appear only when you are not already looking at that agent.

## Settings and keys

- **A toast reports an error in settings.json, keys.json or programs.json.** The toast names the file, line and column. Canopy keeps using the last good version until you fix and save the file. See [Settings and Themes](Settings-and-Themes#errors-in-settings-files).
- **My shortcuts are in a bad state.** Press `Ctrl+Alt+Shift+Esc` to restore the default layout and keys. Canopy keeps your old files as `*.reset-<time>` backups.

## The canopy command

- **`canopy` in a terminal opens a second copy of the app.** The terminal is using a different app home. Check `HERDR_APP_HOME` and `XDG_CONFIG_HOME` ([Files and Folders](Files-and-Folders)). Run `canopy` by name, not by full path, so a terminal inside Canopy hands off to the running app.
- **`canopy ls`, `canopy status` or `canopy do` fails.** These talk to the running window. Open Canopy first.

## Problems with a specific feature

- [Terminals](Terminals#troubleshooting)
- [Agents](Agents#troubleshooting)
- [File and Note Panes](File-and-Note-Panes)
- [Images and PDFs](Images-and-PDFs#troubleshooting)
- [Browser Pane](Browser-Pane#troubleshooting)
- [Simulator Devices](Simulator-Devices#troubleshooting)
- [Machines](Machines#troubleshooting)
- [Agent Setup](Agent-Setup#troubleshooting)

## Report a bug

1. If Canopy opens, keep it open.
2. In a terminal using the same app home, run `canopy status` and copy its output. It describes what the window shows and any errors.
3. Open a [bug report](https://github.com/Krishnapthm/canopy/issues/new?template=bug.yml) with that output, the steps to reproduce, what you expected, your Canopy version (`canopy --version`) and your operating system.

If Canopy cannot open, include the error it shows instead of the `canopy status` output.
