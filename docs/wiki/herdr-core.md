# herdr-core

[herdr-core](https://github.com/herdrdev/herdr-core) is the separate program (`hc`) that runs your terminals and agents.

## What the core does

The core keeps your terminals and agents running when you close the Canopy window.

See [Machines](Machines) for other computers and [Wiring](Wiring) for `hc` messaging.

## Install and update the core

The [Canopy installer](https://github.com/Krishnapthm/canopy#install) installs the core if it is missing.
The Canopy app never installs or updates the core.

To update the core, run:

```sh
hc update
```

The update keeps your terminals running.

## Check the core

Canopy requires core version `0.1.20` or newer, as recorded in [distribution/core-version](https://github.com/Krishnapthm/canopy/blob/main/distribution/core-version).
It also checks that the core supports the protocol and operations it needs.

From the source checkout, `scripts/core-check /absolute/path/to/hc` checks compatibility with a disposable core, then stops it.

## Troubleshooting

- **Canopy needs herdr-core.** Canopy cannot find the core. Follow the displayed install instructions, or click **I have it somewhere** to select its executable.
- **Canopy needs a newer herdr-core.** The core is too old or lacks required operations. Run `hc update`, then click **Retry**.
