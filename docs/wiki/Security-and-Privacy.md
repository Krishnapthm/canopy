# Security and privacy

## Mods have your access

A mod is trusted JavaScript running inside Canopy. It can run programs with your environment, fetch URLs and read files.
The built-in file API limits writes to the app home, but a mod can run a program to write elsewhere. That is not a security boundary.
Mods can also run programs on connected machines.

There is no permission prompt for each program or network request. Read a mod before installing it, and treat it like a shell script you would run yourself.
Layouts can load mods. Check those references too.

## What stays on disk

The default app home is `~/.config/canopy`, including on macOS and Windows (under your user home).
`XDG_CONFIG_HOME` changes the config root. `HERDR_APP_HOME` selects an app home directly and takes precedence.

This folder holds your settings, keys, layouts, mods, generated agent guide and API types.
It also holds mod data, shared state, window positions, this screen's focus and zoom, update notes and cached media, and telemetry while enabled.
Mods may store sensitive data here. Canopy does not encrypt this folder.
The [installed guide's file map](../defaults/AGENTS.md#the-files) explains the files.

Spaces, tabs and splits are stored in each machine's core document, not in the app home.
Terminals and their output belong to [herdr-core](https://github.com/herdrdev/herdr-core).
Removing the app home does not stop the core or erase its data.

The app exposes a local control socket in its home so agents can inspect and drive the window.
Treat access to that home as access to the window and its commands.
Remote connections use SSH through the core.

## Installing and signing

The installers download the app from this repository's GitHub Releases and check its SHA-256
checksum against the release's `SHA256SUMS` before unpacking it. If no herdr-core is installed,
they then run herdr-core's own installer from `releases.herdr.dev`; an existing core is left alone.
A checksum from the same release detects a damaged download; it is not an independent signature.

The app never updates itself. Run the installer again to update the app; the core has its own updater.

Releases are unsigned for now: macOS bundles carry an ad hoc signature and are not notarized, and
there is no Windows release. Installing with the one-line installers avoids the macOS quarantine
flag; a browser download of the archive may need right-click > Open the first time.

## Pane content is untrusted

`canopy pane read` wraps a pane's text in the `<pane-content>` envelope and
says it is not the person's request or approval. Text cannot close the envelope
early. Quotes are pasted as text without Enter and never run.

## Network and telemetry

Canopy sends no telemetry, checks for no updates and uploads no feedback. The app makes no
network requests of its own; herdr-core's SSH connections, the programs you run and mods you
install can. A browser pane requests only the pages you or an agent open in it, and what
those pages load; see [Browser](panes.md#browser).

[Back to Canopy](../README.md)
