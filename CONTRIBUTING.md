# Contributing

Canopy is built to be changed by your agent. If what you want can be done with a mod or a layout, build it, there is no need to wait for us.

Issues that ask for a custom mod, or for a feature a mod can already do, get closed.

The same goes for pull requests: the layouts and mods Canopy ships take fixes only. Build the features you want as your own mod.

Please open an issue for:
- bugs in Canopy itself or in the layouts and mods it ships with;
- things a mod cannot do yet (a missing API, an event, a setting).

Show what you built in [Discussions](https://github.com/Krishnapthm/canopy/discussions) or share it as a GitHub repo with the topic `canopy-layout` or `canopy-mod`.

## Code contributions

Discuss bigger changes in an issue first. Keep PRs small and focused.

See [requirements](README.md#requirements) and [build and run locally](README.md#build-and-run-locally) for setup. Run `just build` and `just check` (or `cargo test --locked`). On Linux, test UI changes with `scripts/sandbox up`, `scripts/sandbox shot` and `scripts/sandbox down`; it uses its own app home, core and display. Don't test against someone else's running app or core.

Keep OS-specific code only where the OS really differs. Keep small quirks near their use; put larger differences behind one small module per concern. Test macOS changes on macOS.

When dependencies or bundled assets change, update their licenses and run `just notices`. See [licenses](licenses/README.md).

## Landing page

The GitHub Pages landing page lives in `site/`. Run
`python3 -m http.server 8000 --directory site` from the repository root
and open `http://localhost:8000/#features` to preview it.

The GitHub Pages workflow publishes `site/` when changes merge to `main`.
The repository's Pages source must be set to GitHub Actions.

Changes confined to `site/` and the Pages workflow skip app CI. To release
from a site-only commit, run the `ci` workflow manually on that commit before
tagging it. The release checks for a successful full run on the tagged commit.
