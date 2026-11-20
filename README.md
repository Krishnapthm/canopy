<p align="center">
  <img src="docs/assets/canopy-header.svg" alt="Canopy: a free canvas of agent and terminal panes joined by wires" width="100%">
</p>

# Canopy

A control room for agents and terminals. Canopy is a native Rust/GPUI desktop
app with persistent terminals powered by the separately installed herdr-core.
The main layout is a free canvas with spaces, terminal and agent cards, a view
filter and terminal pop-outs. The classic layout retains the original spaces and tabs.

Based on herdr-app: https://github.com/herdrdev/herdr-app.
Licensed under Apache-2.0. See LICENSE, NOTICE and THIRD-PARTY-NOTICES.

## Install

macOS (Apple Silicon) and Linux:

```sh
curl -fsSL https://github.com/Krishnapthm/canopy/releases/latest/download/install.sh | sh
```

This installs Canopy and, if you don't have it yet, herdr-core, the terminal engine it runs on.
Then open Canopy from your apps or run `canopy`. Run the installer again to update.

To uninstall, delete the app and its config in `~/.config/canopy`:

- macOS: `~/Applications/Canopy.app` and the `~/.local/bin/canopy` link.
- Linux: `~/.local/share/canopy`, `~/.local/bin/canopy`,
  `~/.local/share/applications/canopy.desktop` and
  `~/.local/share/icons/hicolor/scalable/apps/canopy.svg`.

herdr-core stays, as other apps may use it.

## Build from source

Install Rust and Zig 0.16. On macOS, use Homebrew's keg-only zig@0.16:

```sh
brew install zig@0.16
export PATH="$(brew --prefix zig@0.16)/bin:$PATH"
cargo build --locked
cargo test --locked
```

A source build needs herdr-core too; the installer above sets it up, or run its
own installer:

```sh
curl -fsSL https://releases.herdr.dev/core/install.sh | sh
```

The app never installs or updates the core, collects telemetry, or checks for
updates; only the installers fetch the core. Its config lives in ~/.config/canopy; HERDR_APP_HOME overrides it.
The internal mod API and environment variable names retain their upstream names.

## Development

Check a core without opening the app: `scripts/core-check /absolute/path/to/hc`.
It starts and stops a disposable core in scratch state. See
[the core contract](docs/canopy/core-contract.md) for requirements and limits.

After review, launch an isolated window and core:

```sh
scripts/dev
# Close the development window first, then:
scripts/dev-stop
```

Closing the window leaves the scratch core running, so running `scripts/dev`
again reattaches to it with your layout and terminals; `scripts/dev-stop` ends it.
These scripts use a scratch home, config, core socket and state under
TMPDIR/canopy-dev. Never run a bare debug binary against your daily app home or core.
An older Canopy CLI may already be on PATH; the scripts use target/debug/canopy
directly, and a `canopy` run by name in a dev terminal hands off to it (from
the next installed release on). Builds do not install over that CLI.

just test also runs Bun mod tests. See AGENTS.md for sandbox and platform checks.

## Releasing

1. Set `version` in Cargo.toml, merge to main and wait for ci on main to pass.
2. `git tag v<version> && git push origin v<version>`.

The release workflow builds every platform, waits for that ci run, and publishes a GitHub
Release with the archives, `SHA256SUMS` and both installers. Actions > release > Run workflow
builds everything without publishing.
