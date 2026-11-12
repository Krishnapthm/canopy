# Canopy

A control room for agents and terminals. Canopy is a native Rust/GPUI desktop
app with persistent terminals powered by the separately installed herdr-core.
The main layout is a free canvas with spaces, terminal and agent cards, a view
filter and terminal pop-outs. The classic layout retains the original spaces and tabs.

Based on herdr-app: https://github.com/herdrdev/herdr-app.
Licensed under Apache-2.0. See LICENSE, NOTICE and THIRD-PARTY-NOTICES.

## Build from source

Install Rust and Zig 0.16. On macOS, use Homebrew's keg-only zig@0.16:

```sh
brew install zig@0.16
export PATH="$(brew --prefix zig@0.16)/bin:$PATH"
cargo build --locked
cargo test --locked
```

Install herdr-core yourself using its documented installer:

```sh
curl -fsSL https://releases.herdr.dev/core/install.sh | sh
```

Canopy never installs or updates the core, collects telemetry, or checks for
app updates. Its config lives in ~/.config/canopy; HERDR_APP_HOME overrides it.
The internal mod API and environment variable names retain their upstream names.

## Development

After review, launch an isolated window and core:

```sh
scripts/dev
# Close the development window first, then:
scripts/dev-stop
```

These scripts use a scratch home, config, core socket and state under
TMPDIR/canopy-dev. Never run a bare debug binary against a daily herdr setup.
An older Canopy CLI may already be on PATH; the scripts use target/debug/canopy
directly. Builds do not install over that CLI.

just test also runs Bun mod tests. See AGENTS.md for sandbox and platform checks.
