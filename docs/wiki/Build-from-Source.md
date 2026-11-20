# Build from Source

Build and test Canopy with an isolated app home and core for development.

## Requirements

Follow the [README build instructions](https://github.com/Krishnapthm/canopy#build-from-source) for requirements and build commands.
On macOS, add Homebrew's Zig 0.16 keg to your `PATH`:

```sh
export PATH="$(brew --prefix zig@0.16)/bin:$PATH"
```

See [herdr-core](herdr-core) for core installation.

## Build and test

- `just build` runs `cargo build --locked`.
- `just test` runs the Bun mod tests and `cargo test --locked`.
- `just check` runs the same tests as `just test`.

## The development window

> **Warning:** Never launch a bare debug binary against your daily app home or core.

1. Build Canopy before you run `scripts/dev`.
2. Run `scripts/dev` to open the development window.
3. Close the development window before you run either development script again.
4. Run `scripts/dev-stop` to stop only its reserved scratch core.

The scripts keep the app home, config, core socket and state under `${TMPDIR:-/tmp}/canopy-dev`.
Closing the window leaves the scratch core running.
Run `scripts/dev` again to reattach with your layout and terminals.

## Checks before a pull request

Follow [CONTRIBUTING.md](https://github.com/Krishnapthm/canopy/blob/main/CONTRIBUTING.md) for contribution rules.

1. Run `just build`.
2. Run `just test`.
3. Run `just docs-check` to check generated commands, CLI examples and wiki style.

`just docs-generate` regenerates [Commands](Commands).
Both documentation recipes build Canopy first.

Run `scripts/core-check /absolute/path/to/hc` to check a core without opening Canopy.
It starts and stops a disposable core in scratch state.

On Linux, use `scripts/sandbox up` for a separate app home, core and display, then `scripts/sandbox shot` for a screenshot.
Run `scripts/sandbox down` when you finish.

## Windows checkouts

Canopy embeds text files byte for byte. `.gitattributes` requires LF line endings on every OS.
For Windows clones made before that file existed, refresh the CRLF copies once.

> **Warning:** Run this refresh only from a clean working tree.

```sh
git rm -r --cached -q . && git reset --hard
```
