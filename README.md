<h1 align="center">Canopy</h1>

<p align="center">
  <img src="assets/icons/canopy.svg" width="128" height="128" alt="Canopy icon">
</p>

<p align="center"><b>the canvas your coding agents work on.</b></p>

<p align="center">
  <a href="https://github.com/Krishnapthm/canopy/wiki/Quick-Start">quick start</a> · <a href="https://github.com/Krishnapthm/canopy/wiki">docs</a> · <a href="CONTRIBUTING.md">contributing</a>
</p>

canopy puts your terminals and coding agents side by side on one canvas. wire agents together, see which one is waiting on you, and open files, a browser or an iOS simulator next to them. close the window and everything keeps running.

## Install

canopy runs on macOS (apple silicon) and linux.

```sh
curl -fsSL https://github.com/Krishnapthm/canopy/releases/latest/download/install.sh | sh
```

the installer also sets up herdr-core if you don't have it yet. open canopy from your apps or run `canopy`, then follow the [quick start](https://github.com/Krishnapthm/canopy/wiki/Quick-Start) to set up your first canvas.

canopy never updates itself. to update, run the same command again.

<details>
<summary>uninstall</summary>

delete the app, its command-line link and its config in `~/.config/canopy`.

on macOS the app is `~/Applications/Canopy.app` and the link is `~/.local/bin/canopy`.

on linux, remove `~/.local/share/canopy`, `~/.local/bin/canopy`, `~/.local/share/applications/canopy.desktop` and `~/.local/share/icons/hicolor/scalable/apps/canopy.svg`.

herdr-core is the engine herdr runs on too, so keep it if you use herdr. if you only installed it for canopy, run `hc server stop` to stop it, which ends any terminals still running in it. then delete `~/.local/bin/herdr-core`, `~/.local/bin/hc`, `~/.config/herdr-core` and `~/.local/state/herdr-core`, plus `~/.local/share/herdr-core` on macOS.

</details>

## Requirements

- [rust](https://rustup.rs) 1.95 or newer
- [zig](https://ziglang.org/download/) 0.16
- [herdr-core](https://github.com/herdrdev/herdr-core), which runs your terminals

on linux you also need the system libraries for windowing and fonts. the wiki's [build from source](https://github.com/Krishnapthm/canopy/wiki/Build-from-Source) page lists them.

## Build and run locally

```sh
git clone https://github.com/Krishnapthm/canopy.git
cd canopy
cargo build --locked
scripts/dev
```

`--locked` builds exactly the dependency versions pinned in `Cargo.lock`. `scripts/dev` opens a development window with its own scratch home and core, so it never touches your installed canopy. close the window, then run `scripts/dev-stop`.

## Built on herdr

canopy is a fork of [herdr-app](https://github.com/herdrdev/herdr-app), and every terminal in canopy runs on [herdr-core](https://github.com/herdrdev/herdr-core). both are made by [herdr](https://github.com/herdrdev). if you like canopy, give herdr a try too.

## License

canopy is licensed under Apache-2.0, like herdr-app. see [LICENSE](LICENSE), [NOTICE](NOTICE) and [THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES).

made by [krishna](https://github.com/Krishnapthm).
