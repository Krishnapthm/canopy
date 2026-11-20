# Canopy

Canopy is a desktop app for terminals and coding agents. Put them on one canvas,
connect them with wires, and see which agent needs you.

Terminals keep running when you close the window. Canopy runs them on
[herdr-core](https://github.com/herdrdev/herdr-core).

## Install

macOS (Apple Silicon) and Linux:

```sh
curl -fsSL https://github.com/Krishnapthm/canopy/releases/latest/download/install.sh | sh
```

The installer adds Canopy and, if it is missing, herdr-core.
Open Canopy from your apps or run `canopy`.
Then follow the [Quick start](https://github.com/Krishnapthm/canopy/wiki/Quick-Start).

To update, run the installer again. The app never updates itself.

To uninstall, delete the app and its config in `~/.config/canopy`:

- macOS: `~/Applications/Canopy.app` and the `~/.local/bin/canopy` link.
- Linux: `~/.local/share/canopy`, `~/.local/bin/canopy`,
  `~/.local/share/applications/canopy.desktop` and
  `~/.local/share/icons/hicolor/scalable/apps/canopy.svg`.

herdr-core stays, because other apps can use it.

## Docs

The [wiki](https://github.com/Krishnapthm/canopy/wiki) has every feature, how to use it and how to fix problems.

## Build from source

You need Rust, Zig 0.16 and herdr-core. On macOS:

```sh
brew install zig@0.16
export PATH="$(brew --prefix zig@0.16)/bin:$PATH"
cargo build --locked
cargo test --locked
```

Open an isolated development window with its own home and core:

```sh
scripts/dev
```

Close that window, then stop its core:

```sh
scripts/dev-stop
```

[Build from source](https://github.com/Krishnapthm/canopy/wiki/Build-from-Source) has the details.

## License

Apache-2.0. Based on [herdr-app](https://github.com/herdrdev/herdr-app).
See LICENSE, NOTICE and THIRD-PARTY-NOTICES.
