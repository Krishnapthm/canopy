# Canopy

Run Claude Code and Codex side by side, and let them hand work to each other.

You start an agent with one command. That agent can start helper agents, even ones from a
different vendor ("have Codex write the tests"). Canopy shows them all as one tree, with
what each one is doing and how many tokens it has used.

Everything runs on your machine. Canopy uses the `claude` and `codex` you already have
installed and logged in. It never sees your API keys.

> Status: early. The terminal UI, the web app and the command line work today.

## Before you start

You need:

- **macOS or Linux**
- **Claude Code and/or Codex**, installed and logged in. If `claude` or `codex` works in
  your terminal, you are set.
- **Rust**, to build Canopy. Canopy is written in Rust, but you do not need to know Rust.
  Install it with:

  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

  Then open a new terminal window so it picks up the change.

## Install

Install Node.js 24 or newer as well. From a fresh clone, run these commands
in the repository root, in this order:

```bash
make web
cargo install --path crates/canopy
```

`make web` installs the web build tools and creates the assets that Rust embeds.
Run it before `cargo install`, including when updating.

The first install takes a few minutes. It puts `canopy` in `~/.cargo/bin`, which the Rust
installer already added to your PATH.

Check that it worked:

```bash
canopy doctor
```

The table shows installed versions beside recorded versions and each CLI's login state.
Bare `canopy` prints the same table until the terminal UI lands. OpenCode, Copilot CLI
and Gemini CLI are detected but not supported yet.

Try a disposable agent tree without a subscription or tokens:

```bash
canopy demo
```

The demo starts a temporary foreground daemon and stops it before exiting.

**Seeing `canopy: command not found`?** Open a new terminal window and try again. If it
still fails, run `source ~/.cargo/env` and try once more.

## Your first agent

Go to any project folder and open the control room:

```bash
canopy
```

Press `c`, type `claude explain what this project does`, then Enter. Use `j/k` to
select agents, `i` to message, `a/d` to answer permissions, and `x` to interrupt.
Press `:` for all commands, `?` for help, and `q` to quit. Agents keep running;
open `canopy` again to reattach. Use Page Up and Page Down to scroll output.
Set `CANOPY_NERD_FONT=1` to use the Nerd Font earth icon (requires a patched font).
The default uses plain text symbols.

For scripts, start an agent with:

```bash
canopy run claude "explain what this project does"
```

Canopy prints the agent's id, like `01JD5V8R2N3XK4Y7Q3MZ8KD9TA`. You only need the last 8
characters. Watch it work:

```bash
canopy tail MZ8KD9TA -f
```

`-f` keeps following until the agent finishes. Press Ctrl-C to stop watching (the agent
keeps going).

## Everyday commands

| Command | What it does |
| --- | --- |
| `canopy run claude "..."` | Start a Claude Code agent in the current folder |
| `canopy run codex "..."` | Start a Codex agent |
| `canopy ls` | Show all agents as a tree |
| `canopy tail <id> -f` | Watch an agent's output live |
| `canopy send <id> "..."` | Send a follow-up message |
| `canopy stop <id>` | Stop an agent and every helper it started |
| `canopy web` | Open the local web control room |
| `canopy doctor` | Check that Claude Code and Codex are ready |
| `canopy help` | List every command and option |

## Agents helping agents

Every agent Canopy starts gets four extra tools: `spawn_agent`, `wait_agent`,
`message_agent`, and `list_agents`. Just ask for it in plain words:

```bash
canopy run claude "add a --json flag to the CLI. use spawn_agent to have codex write the tests"
```

Then run `canopy ls` to see the parent and its helper in one tree.

## Permissions

By default an agent asks before editing files or running commands. When it asks,
`canopy tail` shows a line like:

```
permission: Edit src/main.rs [request 01JD...]
```

Answer it with:

```bash
canopy permit <id> <request> allow     # or deny, or always
```

To ask less often, pick a policy when you start the agent:

```bash
canopy run --policy auto_edit codex "fix the lint errors"
```

| Policy | The agent... |
| --- | --- |
| `ask` (default) | asks before edits and commands |
| `auto_edit` | edits files freely, still asks before risky commands |
| `yolo` | never asks. Only use this in a folder you can throw away |

Helper agents get the same policy as their parent and can never get more.

## Good to know

- **Short ids.** Anywhere you need an id, the 8 characters `canopy ls` shows are enough.
- **The background service.** Canopy runs a small background service (the "daemon") that
  keeps your agents alive. It starts on its own when you run a command and shuts down
  after 10 idle minutes. You normally never touch it.
- **Where data lives.** In `~/.canopy`. Set `CANOPY_HOME` to use a different folder.
- **Limits.** By default helpers can nest 3 levels deep and 16 agents can run at once.
  Change this with `canopy daemon --max-depth 4 --max-live 8`.
- **Other agents.** Any agent that speaks ACP can be added by name, for example
  `canopy daemon --acp gemini="gemini --acp"`, then `canopy run gemini "..."`.
- **Stopping everything.** `pkill -f "canopy daemon"` stops the service and every agent.

## Updating

From inside the `canopy` folder:

```bash
git pull
```

```bash
make web
cargo install --path crates/canopy
```

## Working on Canopy

You need [Node.js](https://nodejs.org) 24 or newer as well. Then:

```bash
make check    # format and lint
```

```bash
make test     # fast tests
```

```bash
make e2e      # full tests with fake agents, no real tokens spent
```

New here? Read [AGENTS.md](AGENTS.md) for the rules of the codebase and
[PLAN.md](PLAN.md) for the design.

Agents share the parent's project folder, so helpers can edit the same files. Each harness
loads your own plugins, hooks and MCP servers. Canopy does not change your login or read
credentials; it drives the official CLI and uses its status command to check login.

To uninstall, remove the `canopy` binary from `~/.cargo/bin` and delete `~/.canopy`.

## License

MIT

## Web development

Build the web app with `make web`. It checks the 160 KB workspace first-load JavaScript gzip budget.
The web app follows your system light or dark theme. Use Outline or Graph to switch
views, and press `?` for keyboard help or `Cmd-K` for commands.
`make build` builds the app and embeds it in the release binary.

For a development preview, use two terminals with the same temporary CANOPY_HOME.
Start `canopy daemon --foreground`, then `canopy web` to print the local URL.
From the web directory, run `CANOPY_WEB_ORIGIN=http://127.0.0.1:PORT npm exec vite`,
using the printed port. Open Vite's local URL with the same #token fragment
from the printed Canopy URL. Stop Vite and the foreground daemon
with Ctrl-C when done. Normal use needs only `canopy web`.

In the web app, press `g p` or choose Plugins and skills in the command palette.
The list includes global items and project items for the selected agent. Use the
harness switches to change an item or Adopt to move a vendor skill into the library.
Opening the view or changing an item reads a fresh list.

In the TUI, press `g p` to open plugins and skills for the current project. Use the
arrow keys to select an item, Space to switch it, and `r` to read the list again.
Changes apply to newly started agents. Running agents keep what they loaded.
Place shared skills in `$CANOPY_HOME/skills/<name>/SKILL.md` (by default,
`~/.canopy/skills/`). Switch one on to link it into a harness's skill directory.
Press `a` on an existing vendor skill to adopt its folder into the shared library.
Canopy never installs plugins or executes marketplace commands. Vendor extension
interfaces and shared skill loading still need live verification; unsupported
commands appear as errors in the view.
