# herdr-core

[herdr-core](https://github.com/herdrdev/herdr-core) is the small background program that runs your terminals and agents. Its command is `hc`. Canopy is a window onto it: the window draws terminals and sends your keystrokes, but the programs themselves live in the core.

## Why Canopy uses a core

Splitting the window from the terminals is what makes Canopy dependable for long-running agents:

- **Your agents survive the window.** Close Canopy, update it, or let it crash, and every terminal keeps running. Open Canopy again and everything is where you left it.
- **Other machines work the same way.** A server runs its own core, and Canopy shows its terminals next to yours. Those agents keep working while your laptop sleeps. See [Machines](Machines).
- **Terminals can drive each other.** Any terminal can read another's screen, send it a line or wait for it with `hc`. This is how agents talk to their peers. See [Wiring](Wiring#talk-to-peers).

Terminals stop when you close them in Canopy, when their program exits, or when the computer shuts down.

## Useful hc commands

You do not need `hc` to use Canopy, but it is handy in a terminal:

```sh
hc ls              # list running terminals with their addresses (t1, t2, ...)
hc attach t3       # put this terminal on t3, from any terminal app (ctrl+\ detaches)
hc read t3         # print t3's screen
hc --help          # everything else
```

## Install and update the core

The [Canopy installer](https://github.com/Krishnapthm/canopy#install) installs the core if it is missing. The Canopy app itself never installs or updates the core.

To update the core, run:

```sh
hc update
```

Your terminals keep running through the update.

## Version requirements

Canopy needs core version `0.1.20` or newer, as recorded in [distribution/core-version](https://github.com/Krishnapthm/canopy/blob/main/distribution/core-version). On start, it also checks that the core supports the protocol and operations it uses.

From a source checkout, `scripts/core-check /absolute/path/to/hc` checks a core's compatibility using a throwaway core, then stops it.

## Troubleshooting

- **Canopy says it needs herdr-core.** Canopy cannot find the core. Follow the install instructions it shows, or click **I have it somewhere** to point it at the `hc` executable.
- **Canopy says it needs a newer herdr-core.** The core is too old or lacks something Canopy needs. Run `hc update`, then click **Retry**.

## See also

- [Concepts](Concepts#herdr-core-why-nothing-dies-with-the-window): the window and core split in context.
- [Labels](Labels): how core clients share seen and pinned state.
