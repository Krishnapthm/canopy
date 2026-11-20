# CLI

Use `canopy` to open the app, inspect its window or run app commands from a terminal.

| Command | What it does |
|---|---|
| `canopy` | Opens the window, or brings the running window to the front. |
| `canopy ls` | Prints the open window's spaces, tabs and terminals as JSON. |
| `canopy do <command> [args]` | Runs an app command. See [Commands](Commands). |
| `canopy commands` | Prints app commands, their arguments and descriptions as JSON, without an open window. |
| `canopy status` | Prints what the window shows and any errors as JSON. |
| `canopy shot [file.png]` | Saves the last window frame as a PNG and prints its path. The default is `canopy-shot.png`. |
| `canopy hook <event>` | Used by agent session hooks. |
| `canopy wire` | Lists your wired peers and panes. See [Wiring](Wiring). |
| `canopy pane <verb> [args]` | Controls canvas panes. See [Agent Pane Commands](Agent-Pane-Commands). |
| `canopy spawn <program> --name <n> [--model <m>] [-- <message>]` | Starts a helper agent. See [Sub-Agents](Sub-Agents). |
| `canopy check <mod folder>` | Loads a mod once and reports errors. |
| `canopy config` | Prints settings defaults and explanations. |
| `canopy --skill` | Prints the Canopy skill. |
| `canopy skill remove` | Removes the skill Canopy installed. See [Agent Setup](Agent-Setup). |
| `canopy --version` | Prints the Canopy version. |
| `canopy --help` | Prints CLI help. |

## Examples

Open the app before using `ls`, `status`, `shot` or `do`.

```sh
canopy ls                       # inspect spaces, tabs and terminals
canopy status                   # inspect the window and errors
canopy shot window.png           # save a window screenshot
canopy commands                 # list app commands and arguments
```

Pass `do` arguments as `--arg value` pairs or one JSON object.
Values that parse as JSON numbers, booleans, arrays or objects keep those types.

```sh
canopy do canvas.arrange --layout row
canopy do canvas.arrange '{"layout":"row"}'
```
