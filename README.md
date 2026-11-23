<h1><img src="assets/icons/canopy.svg" width="128" height="128" align="left" alt="Canopy icon">Canopy<br clear="left"></h1>

Canopy gives your terminals and coding agents one big canvas to live on. Put them wherever makes sense to you, wire agents together so they can talk, and glance over to see which one is waiting on you. It's not just shells, either: open the file you're working on, a browser on your dev server or an iOS simulator right next to the agent building it, so you both see the same thing. Your terminals run in [herdr-core](https://github.com/herdrdev/herdr-core), so closing the window doesn't kill them.

[Quick start](https://github.com/Krishnapthm/canopy/wiki/Quick-Start) · [Docs](https://github.com/Krishnapthm/canopy/wiki) · [Contributing](CONTRIBUTING.md)

## Install

Canopy runs on macOS (Apple Silicon) and Linux.

```sh
curl -fsSL https://github.com/Krishnapthm/canopy/releases/latest/download/install.sh | sh
```

The installer also sets up herdr-core if you don't have it yet. Open Canopy from your apps or run `canopy`, then follow the [Quick start](https://github.com/Krishnapthm/canopy/wiki/Quick-Start) to set up your first canvas.

Canopy never updates itself. To update, run the same command again.

<details>
<summary>Uninstall</summary>

Delete the app, its command-line link and its config in `~/.config/canopy`.

On macOS the app is `~/Applications/Canopy.app` and the link is `~/.local/bin/canopy`.

On Linux, remove `~/.local/share/canopy`, `~/.local/bin/canopy`, `~/.local/share/applications/canopy.desktop` and `~/.local/share/icons/hicolor/scalable/apps/canopy.svg`.

herdr-core stays installed, because other apps can use it.

</details>

## Build from source

You need [Rust](https://rustup.rs) 1.95 or newer, [Zig](https://ziglang.org/download/) 0.16 to build the terminal engine, and [herdr-core](https://github.com/herdrdev/herdr-core) to run your terminals. [just](https://github.com/casey/just) and [Bun](https://bun.sh) are optional and only needed for the full test suite.

<details>
<summary>Set up on macOS</summary>

With the Xcode Command Line Tools and [Homebrew](https://brew.sh):

```sh
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
brew install zig@0.16 just oven-sh/bun/bun
curl -fsSL https://releases.herdr.dev/core/install.sh | sh
```

Homebrew's `zig@0.16` is keg-only, so add it to your `PATH` in your shell profile:

```sh
export PATH="$(brew --prefix zig@0.16)/bin:$PATH"
```

</details>

<details>
<summary>Set up on Linux (Debian or Ubuntu)</summary>

Put Zig 0.16 from [ziglang.org](https://ziglang.org/download/) on your `PATH`, then:

```sh
sudo apt-get install build-essential pkg-config clang libclang-dev \
  libxcb1-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
  libvulkan-dev mesa-vulkan-drivers libfontconfig-dev libfreetype-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
curl -fsSL https://releases.herdr.dev/core/install.sh | sh
```

</details>

Then clone, build and test:

```sh
git clone https://github.com/Krishnapthm/canopy.git
cd canopy
cargo build --locked
cargo test --locked
```

`just test` runs the same tests plus the Bun mod and script tests.

To try your build, run `scripts/dev`. It opens a development window with its own scratch home and core, so it never touches your installed Canopy. When you're done, close the window and run `scripts/dev-stop` to stop its core. Don't run the debug binary directly, because it would use your real app home and core.

The wiki's [Build from source](https://github.com/Krishnapthm/canopy/wiki/Build-from-Source) page covers the checks to run before a pull request, the Linux sandbox and Windows checkouts.

## Credits and license

Canopy began as a fork of [herdr-app](https://github.com/herdrdev/herdr-app), and every terminal you open in it runs on herdr-core. Huge thanks to [herdr](https://github.com/herdrdev) for building that foundation in the open.

Canopy is licensed under Apache-2.0, like herdr-app. See [LICENSE](LICENSE), [NOTICE](NOTICE) and [THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES).
