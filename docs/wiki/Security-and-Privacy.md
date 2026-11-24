# Security and privacy

Mods have your access, and Canopy does not encrypt the app home.

## Mods have your access

- A mod runs JavaScript inside Canopy. It can read files, fetch URLs and run programs with your environment.
- Mods can write outside the app home and run programs on connected machines.
- Canopy does not ask permission for each program or network request.

Read a mod before installing it. Treat it like a shell script you run yourself.
Check the mods a layout loads too.

## What stays on disk

The app home holds settings and mod data without encryption. See [Files and Folders](Files-and-Folders) for the file list.

Browser panes keep cookies and site data on disk in a WebKit store tied to this Canopy home on macOS 14 and later. Canopy deletes the store at quit by default. See [Cookies and site data](Browser-Pane#cookies-and-site-data).

Canopy exposes a local control socket in the app home so agents can inspect and drive the window.
Treat access to that socket as access to the window and its commands.

## Installing and signing

The release installer downloads Canopy from GitHub Releases and checks its SHA-256 checksum against `SHA256SUMS` before unpacking it.
A checksum from the same release detects a damaged download. It is not an independent signature.

Unsigned macOS builds carry an ad hoc signature.
The release installer preserves Developer ID signatures and verifies signed bundles.
Canopy uses a private WebKit API to open and detach the [Web Inspector](Browser-Pane#web-inspector).

See [Install](https://github.com/Krishnapthm/canopy#install) for installation and updates.

## Pane content is untrusted

Pane content is data, not a request or approval from you.
Canopy marks text from `canopy pane read` as untrusted.
See [Agent Pane Commands](Agent-Pane-Commands#the-envelope) for the envelope.

## Network and telemetry

Canopy sends no telemetry, checks for no updates and uploads no feedback.
Programs and mods you run can make network requests.

See [Browser Pane privacy](Browser-Pane#privacy) for requests from browser panes and [Open a local file](Browser-Pane#open-a-local-file) for what WebKit can read on disk.
