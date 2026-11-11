# Canopy

Run Claude Code and Codex side by side, and let them hand work to each other.

You start an agent in the control room. That agent can start helper agents, even ones from a
different vendor ("have Codex write the tests"). Canopy shows them all as one tree, with
what each one is doing and how many tokens it has used.

Everything runs on your machine. Canopy uses the `claude` and `codex` you already have
installed and logged in. It never sees your API keys.

> Status: early. The terminal control room and the web app work today.

## Before you start

You need:

- **macOS or Linux**
- **Claude Code and/or Codex**, installed and logged in. If `claude` or `codex` works in
  your terminal, you are set.
- **Node.js 24 or newer**, to build the web app.
- **Rust**, to build Canopy. You do not need to know Rust. Install it with:

  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

  Then open a new terminal window so it picks up the change.

## Install

From the repository root, run these two commands in this order:

```bash
make web
```

```bash
cargo install --path crates/canopy
```

`make web` builds the web app that `cargo install` puts inside the `canopy` binary. The first
install takes a few minutes. It puts `canopy` in `~/.cargo/bin`, which the Rust installer
already added to your PATH.

Check that it worked:

```bash
canopy doctor
```

The table shows each agent CLI Canopy found, its version, and whether you are logged in.

**Seeing `canopy: command not found`?** Open a new terminal window and try again. If it
still fails, run `source ~/.cargo/env` and try once more.

Want to look around first, without spending tokens? Run:

```bash
canopy demo
```

It opens the control room on a tree of fake agents. Press `q` to quit; the demo cleans up
after itself.

## Your first agent

Go to any project folder and open the control room:

```bash
canopy
```

Press `c`, or click **+ New agent** at the bottom. A form opens:

1. Pick **Claude Code** or **Codex** with the arrow keys or the mouse.
2. The directory is the folder you are in. Change it if you like.
3. Type a task, such as `explain what this project does`.
4. Press Ctrl+Enter, or click **Start**.

The agent appears in the tree and its output streams on the right. Everything works with
the mouse: click an agent to open it, click the buttons at the bottom, scroll with the wheel.
The keys, if you prefer them:

| Key | Does |
| --- | --- |
| `j` / `k` | Select the next or previous agent |
| `i` | Send the selected agent a message |
| `a` / `d` | Allow or deny what it asks to do |
| `x` | Interrupt it |
| `r` | Resume an agent that stopped when Canopy restarted |
| `:` | Every command, searchable |
| `?` | Help |
| `q` | Quit. Your agents keep running; run `canopy` again to come back. |

## Running here

The tree also lists the Claude Code sessions already running in this folder outside Canopy,
under **Running here**. They are read-only: Canopy shows their status (idle, busy, or
waiting for you) but never types into them. To continue one, go back to its terminal. For a
background session, the row tells you the exact `claude attach` command. Press `R` to refresh
the list.

Codex sessions running outside Canopy are not listed yet.

## Open in a browser

```bash
canopy web
```

This opens the same control room in your browser. It runs until you press Ctrl+C in that
terminal; your agents keep running after it stops. Only one `canopy web` runs at a time;
a second one prints the address of the first.

The address holds a private access token. Keep it to yourself. Press `?` in the browser for
keys, or Cmd+K for every command.

## When something fails

- **A message pops up.** Every error shows up as a short message in the corner, in the
  terminal and in the browser. It says what went wrong and, where it can, what to do.
- **An agent failed to start or crashed.** Its row shows the reason underneath, for example
  that the CLI is not installed or not logged in. Fix that, then start a new agent.
- **Nothing happens at all.** Run `canopy doctor`. It tells you which CLIs Canopy found and
  whether you are logged in.
- **Still stuck.** The background service writes details to `~/.canopy/daemon.log`.

## Agents helping agents

Every agent Canopy starts gets four extra tools: `spawn_agent`, `wait_agent`,
`message_agent`, and `list_agents`. Ask for them in plain words, for example "add a --json
flag to the CLI. Use spawn_agent to have Codex write the tests". The parent and its helper
appear in one tree.

By default an agent asks before editing files or running commands. Helper agents get the
same rules as their parent and can never get more.

Agents share the parent's project folder, so their edits can overwrite each other. Use git
worktrees when agents should edit independently.

## Plugins and skills

Press `g p` (in the terminal or the browser) to see the plugins and skills each agent CLI
has, and switch them on or off. Changes apply to agents you start afterwards. Canopy never
installs plugins on its own.

## Good to know

- **Short ids.** Canopy shows the last 8 characters of each agent id.
- **The background service.** Canopy runs a small background service that keeps your agents
  alive. It starts on its own and stops after 10 idle minutes. You normally never touch it.
- **Where data lives.** In `~/.canopy`. Set `CANOPY_HOME` to use a different folder.
- **Limits.** Helpers nest at most 3 levels deep, and at most 16 agents run at once.
- **Stopping everything.** `pkill -f "canopy daemon"` stops the service and every agent.
- **Your setup still loads.** Each agent CLI loads your own plugins, hooks and MCP servers.
  Canopy does not change your login or read your credentials.

## Updating

From inside the `canopy` folder:

```bash
git pull
```

```bash
make web
```

```bash
cargo install --path crates/canopy
```

To uninstall, remove `~/.cargo/bin/canopy` and delete `~/.canopy`.

## Working on Canopy

```bash
make check test e2e
```

Read [AGENTS.md](AGENTS.md) for the rules of the codebase and the other `make` targets, and
[PLAN.md](PLAN.md) for the design.

## License

MIT
