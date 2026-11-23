<h1><img src="assets/icons/canopy.svg" width="128" height="128" align="left" alt="Canopy icon">Canopy</h1>

Canopy puts your terminals and coding agents side by side on one canvas.<br>
Wire agents together, see which one is waiting on you, and open files, a browser<br>
or an iOS simulator next to them. Close the window and everything keeps running.

<br clear="left">

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

herdr-core is the engine herdr runs on too, so keep it if you use herdr. If you only installed it for Canopy, run `hc server stop` to stop it, which ends any terminals still running in it. Then delete `~/.local/bin/herdr-core`, `~/.local/bin/hc`, `~/.config/herdr-core` and `~/.local/state/herdr-core`, plus `~/.local/share/herdr-core` on macOS.

</details>

## Build from source

You need [Rust](https://rustup.rs) 1.95 or newer, [Zig](https://ziglang.org/download/) 0.16 to build the terminal engine, and [herdr-core](https://github.com/herdrdev/herdr-core) to run your terminals. [just](https://github.com/casey/just) and [Bun](https://bun.sh) are optional and only needed for the full test suite.

<details>
<summary>Set up on macOS</summary>

These steps use the Xcode Command Line Tools and [Homebrew](https://brew.sh).

```sh
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
brew install zig@0.16 just oven-sh/bun/bun
curl -fsSL https://releases.herdr.dev/core/install.sh | sh
```

Homebrew's `zig@0.16` is keg-only, so add this line to your shell profile to put it on your `PATH`.

```sh
export PATH="$(brew --prefix zig@0.16)/bin:$PATH"
```

</details>

<details>
<summary>Set up on Linux (Debian or Ubuntu)</summary>

Put Zig 0.16 from [ziglang.org](https://ziglang.org/download/) on your `PATH`, then install the rest.

```sh
sudo apt-get install build-essential pkg-config clang libclang-dev \
  libxcb1-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
  libvulkan-dev mesa-vulkan-drivers libfontconfig-dev libfreetype-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
curl -fsSL https://releases.herdr.dev/core/install.sh | sh
```

</details>

Then clone, build and test.

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

Canopy is built on top of [herdr](https://github.com/herdrdev). It started as a fork of [herdr-app](https://github.com/herdrdev/herdr-app), and every terminal you open in Canopy still runs on herdr-core, which is why your agents keep working after the window closes. We never had to write a terminal runtime from scratch because herdr had already built a good one and shared it. herdr is a great app in its own right too, so give it a try if you haven't. Thank you, herdr.

Canopy is licensed under Apache-2.0, like herdr-app. See [LICENSE](LICENSE), [NOTICE](NOTICE) and [THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES).
