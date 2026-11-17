# Canopy changelog

Canopy is based on [herdr-app](https://github.com/herdrdev/herdr-app); its history up to the fork is in that repository.

## [0.1.0] - 2026-11-15

The first Canopy release.

### New
- **A free canvas for agents and terminals.** Spaces become rooms on a Figma-style canvas, with terminal and agent cards, routed edges between them and a view filter. The classic spaces-and-tabs layout is still there. (#3)
- **Agent teams.** Agents on the canvas can message each other through Wire. (#3)
- **Tile view, full-size terminal cells and a shared UI kit for mods.** (#4)
- **The file panel can fill the window,** with the canvas controls beside it. (#6)
- **Install from GitHub Releases** on Linux (x86_64, arm64) and macOS. The installer adds herdr-core when none is installed. (#5, #7)

### Removed
- Telemetry, the self-updater, feedback uploads and in-app release notes. Canopy makes no network requests of its own.
