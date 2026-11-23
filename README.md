<p align="center">
  <img src="assets/icons/canopy.svg" width="128" height="128" alt="Canopy icon">
</p>

<h1 align="center">Canopy</h1>

<p align="center"><b>the canvas your coding agents work on.</b></p>

<p align="center">
  <a href="https://github.com/Krishnapthm/canopy/wiki/Quick-Start">quick start</a> · <a href="https://github.com/Krishnapthm/canopy/wiki">docs</a> · <a href="CONTRIBUTING.md">contributing</a>
</p>

canopy puts your terminals and coding agents side by side on one canvas. wire agents together, see which one is waiting on you, and open files, a browser or an ios simulator next to them. close the window and everything keeps running.

## Install

canopy runs on macos (apple silicon) and linux.

```sh
curl -fsSL https://github.com/Krishnapthm/canopy/releases/latest/download/install.sh | sh
```

the installer also sets up herdr-core if you don't have it yet. open canopy from your apps or run `canopy`, then follow the [quick start](https://github.com/Krishnapthm/canopy/wiki/Quick-Start) to set up your first canvas.

canopy never updates itself. to update, run the same command again.

<details>
<summary>uninstall</summary>

delete the app, its command-line link and its config in `~/.config/canopy`.

on macos the app is `~/Applications/Canopy.app` and the link is `~/.local/bin/canopy`.

on linux, remove `~/.local/share/canopy`, `~/.local/bin/canopy`, `~/.local/share/applications/canopy.desktop` and `~/.local/share/icons/hicolor/scalable/apps/canopy.svg`.

herdr-core is the engine herdr runs on too, so keep it if you use herdr. if you only installed it for canopy, run `hc server stop` to stop it, which ends any terminals still running in it. then delete `~/.local/bin/herdr-core`, `~/.local/bin/hc`, `~/.config/herdr-core` and `~/.local/state/herdr-core`, plus `~/.local/share/herdr-core` on macos.

</details>

## Build from source

you need [rust](https://rustup.rs) 1.95 or newer, [zig](https://ziglang.org/download/) 0.16 to build the terminal engine, and [herdr-core](https://github.com/herdrdev/herdr-core) to run your terminals. [just](https://github.com/casey/just) and [bun](https://bun.sh) are optional and only needed for the full test suite.

<details>
<summary>set up on macos</summary>

these steps use the xcode command line tools and [homebrew](https://brew.sh).

```sh
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
brew install zig@0.16 just oven-sh/bun/bun
curl -fsSL https://releases.herdr.dev/core/install.sh | sh
```

homebrew's `zig@0.16` is keg-only, so add this line to your shell profile to put it on your `PATH`.

```sh
export PATH="$(brew --prefix zig@0.16)/bin:$PATH"
```

</details>

<details>
<summary>set up on linux (debian or ubuntu)</summary>

put zig 0.16 from [ziglang.org](https://ziglang.org/download/) on your `PATH`, then install the rest.

```sh
sudo apt-get install build-essential pkg-config clang libclang-dev \
  libxcb1-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
  libvulkan-dev mesa-vulkan-drivers libfontconfig-dev libfreetype-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
curl -fsSL https://releases.herdr.dev/core/install.sh | sh
```

</details>

then clone, build and test.

```sh
git clone https://github.com/Krishnapthm/canopy.git
cd canopy
cargo build --locked
cargo test --locked
```

`just test` runs the same tests plus the bun mod and script tests.

to try your build, run `scripts/dev`. it opens a development window with its own scratch home and core, so it never touches your installed canopy. when you're done, close the window and run `scripts/dev-stop` to stop its core. don't run the debug binary directly, because it would use your real app home and core.

the wiki's [build from source](https://github.com/Krishnapthm/canopy/wiki/Build-from-Source) page covers the checks to run before a pull request, the linux sandbox and windows checkouts.

## Credits and license

canopy is built on top of [herdr](https://github.com/herdrdev). it started as a fork of [herdr-app](https://github.com/herdrdev/herdr-app), and every terminal you open in canopy still runs on herdr-core, which is why your agents keep working after the window closes. we never had to write a terminal runtime from scratch because herdr had already built a good one and shared it. herdr is a great app in its own right too, so give it a try if you haven't. thank you, herdr.

canopy is licensed under apache-2.0, like herdr-app. see [LICENSE](LICENSE), [NOTICE](NOTICE) and [THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES).
