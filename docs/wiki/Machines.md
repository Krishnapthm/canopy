# Machines

A machine is a computer with herdr-core that Canopy reaches over SSH.

## Add a machine

Install [herdr-core](herdr-core) on the remote computer first.

1. Click **Add a machine** in the welcome, or **Add machine** at the bottom of the **Classic** layout's sidebar.
2. Click **Add** beside a host from `~/.ssh/config`.

For a host outside the list, type its SSH address under **Or type how ssh reaches it**.
Click **Add**.

You can also add a machine from a terminal:

```sh
hc machine add user@host
```

## Use a machine

- The **Classic** layout's sidebar groups spaces by machine when a remote machine is known.
- Click **+** beside a connected machine to create a space there. See [Spaces](Spaces).
- Terminal addresses include the machine name: `mac/t7` is terminal `t7` on machine `mac`.

## Troubleshooting

- **Adding the machine needs a password or a host key answer.** The dialog tells you to run `hc machine add <host>` in a terminal.
  Run that command to answer the prompts.
- **The remote needs herdr-core.** Follow [herdr-core](herdr-core) for installation. The dialog gives the same terminal command when the core is missing or incompatible.
- **A card says "`name` is a Mac: allow its folders once".** macOS asks on that Mac's screen before an agent reads Desktop, Documents or Downloads.
  Click **Open the setting on `name`**. Press **+** and add the herdr-core file the card shows, to give it Full Disk Access.
- **The dialog says agents stop when you disconnect.** The server ends your programs when your last login closes. Run the `sudo loginctl enable-linger` command from the dialog on that machine once.
