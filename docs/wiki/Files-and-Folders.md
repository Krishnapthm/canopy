# Files and Folders

The app home holds Canopy's settings and saved files.

## App home

The default is `~/.config/canopy` on every operating system, including macOS and Windows.
A nonempty `XDG_CONFIG_HOME` changes this to `$XDG_CONFIG_HOME/canopy`.
A nonempty `HERDR_APP_HOME` selects the app home directly and takes precedence.

See [Security and Privacy](Security-and-Privacy) for sensitive data and access to these files.

## What Canopy keeps

Paths below are relative to the app home.

| Path | Contents |
|---|---|
| `settings.json` | Your preferences. See [Settings and Themes](Settings-and-Themes). |
| `keys.json` | Your shortcuts. See [Keys and Mouse](Keys-and-Mouse). |
| `layouts/<name>.json` | Window layouts. See [Layouts and Mods](Layouts-and-Mods). |
| `layouts/.state/` | Saved layout adjustments. |
| `mods/<name>/` | Mod code. See [Layouts and Mods](Layouts-and-Mods). |
| `programs.json` | Program definitions that override the built-in table. See [Agents](Agents). |
| `canvas.json` | Saved canvas data. See [Panes](Panes). |
| `shared.json` | State shared between mods. |
| `data/<name>/` | A mod's saved data. |
| `view.json` | This screen's focus, zoom and recent spaces. |
| `windows.json` | Positions of mod windows and which ones stay open. |
| `browser/<pane>.png` | The last browser pane snapshot, kept between runs. |
| `AGENTS.md`, `herdr.d.ts`, `gpui-kit.d.ts` | The installed guide and API types. |

Each machine keeps its spaces, tabs and splits in its core's `herdr-app` document, outside the app home.
See [herdr-core](herdr-core) for terminal data.

## Temporary files

`<temp>` means the operating system's temporary folder.

- `<temp>/canopy/` holds image quote crops and pane screenshots.
- `<temp>/canopy/sim-chrome/` holds cached simulator bezel images.

## Environment variables

- `HERDR_APP_HOME`: Selects the app home directly, before `XDG_CONFIG_HOME`.
- `CANOPY_SKILL_HOME`: Replaces `~` when Canopy resolves agent skill folders. See [Agent Setup](Agent-Setup).
- `HC_TERMINAL_ID`: Your core terminal address, used to identify the caller for `--wire`.
